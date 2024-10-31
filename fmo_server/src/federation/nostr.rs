use std::collections::HashMap;
use std::time::Duration;

use anyhow::{ensure, Context};
use deadpool_postgres::GenericClient;
use fedimint_core::config::FederationId;
use fedimint_core::encoding::Encodable;
use fedimint_core::task::sleep;
use fmo_api_types::FederationRating;
use nostr_sdk::{
    Event, Filter, FilterOptions, Kind, RelayOptions, RelayPool, RelayPoolOptions,
    RelaySendOptions, SingleLetterTag,
};
use postgres_from_row::FromRow;
use regex::Regex;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::federation::observer::FederationObserver;
use crate::util::{query, query_one};

#[derive(Debug, Clone, FromRow)]
struct NostrRelay {
    relay_url: String,
}

impl FederationObserver {
    /// Syncs Nostr events:
    ///   * Fedimint federation votes
    pub async fn sync_nostr_events(self) {
        const SLEEP_SECS: u64 = 60;
        loop {
            let e = self
                .sync_nostr_events_inner()
                .await
                .expect_err("Not expected to exit");
            warn!("Error while syncing nostr events: {e:?}");
            sleep(Duration::from_secs(SLEEP_SECS)).await;
        }
    }

    async fn nostr_relay_client(&self) -> anyhow::Result<RelayPool> {
        let relays = query::<NostrRelay>(
            &self.connection().await?,
            "SELECT relay_url FROM nostr_relays",
            &[],
        )
        .await?
        .into_iter()
        .map(|relay| relay.relay_url)
        .collect::<Vec<_>>();
        let client = RelayPool::new(RelayPoolOptions::default());
        for relay_url in &relays {
            client.add_relay(relay_url, RelayOptions::default()).await?;
        }
        client.connect(Some(Duration::from_secs(5))).await;

        info!(?relays, "Started Nostr client");

        Ok(client)
    }

    async fn sync_nostr_events_inner(&self) -> anyhow::Result<()> {
        let mut interval = interval(Duration::from_secs(60));

        let client = self.nostr_relay_client().await?;

        loop {
            interval.tick().await;

            let federations = self.list_federations().await?;
            let federation_tag = SingleLetterTag::from_char('d').expect("Tag is valid");

            // TODO: fetch multiple pages till synced up, ok enough for now since new events
            // will always be at the top and old ones will be ignored by us
            let events = client
                .get_events_of(
                    vec![Filter {
                        ids: None,
                        authors: None,
                        kinds: Some(vec![Kind::Custom(38000)].into_iter().collect()),
                        search: None,
                        since: None,
                        until: None,
                        limit: None,
                        generic_tags: HashMap::from([(
                            federation_tag,
                            federations
                                .iter()
                                .map(|federation| federation.federation_id.to_string())
                                .collect(),
                        )]),
                    }],
                    Duration::from_secs(30),
                    FilterOptions::default(),
                )
                .await?;

            info!("Fetched {} nostr events", events.len());

            let mut connection = self.connection().await?;
            let dbtx = connection.transaction().await?;

            let parsed_events = events.into_iter().filter_map(|event| {
                let parsed = ParsedEvent::try_from(event.clone()).ok()?;
                Some((parsed, event))
            });

            for (parsed_event, event) in parsed_events {
                insert_parsed_event(&dbtx, parsed_event, event).await?;
            }

            dbtx.commit().await?;
        }
    }

    pub async fn federation_rating(
        &self,
        federation_id: FederationId,
    ) -> anyhow::Result<FederationRating> {
        #[derive(Debug, Clone, FromRow)]
        struct FederationRatingRow {
            count: i64,
            avg: Option<f64>,
        }

        let query_res = query_one::<FederationRatingRow>(
            &self.connection().await?,
            // language=postgresql
            "SELECT COUNT(star_vote)::bigint as count, AVG(star_vote)::DOUBLE PRECISION as avg from nostr_votes WHERE federation_id = $1;",
            &[&federation_id.consensus_encode_to_vec()],
        )
        .await?;

        Ok(FederationRating {
            count: query_res.count as u64,
            avg: query_res.avg,
        })
    }

    pub async fn submit_rating(&self, nostr_event: Event) -> anyhow::Result<()> {
        let parsed = ParsedEvent::try_from(nostr_event.clone())?;
        let client = self.nostr_relay_client().await?;

        client
            .send_event(
                nostr_event.clone(),
                RelaySendOptions::default().timeout(Some(Duration::from_secs(5))),
            )
            .await?;

        let mut conn = self.connection().await?;
        let dbtx = conn.transaction().await?;
        insert_parsed_event(&dbtx, parsed, nostr_event).await?;
        dbtx.commit().await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ParsedEvent {
    event_id: [u8; 32],
    federation_id: FederationId,
    star_vote: Option<u8>,
}

impl TryFrom<Event> for ParsedEvent {
    type Error = anyhow::Error;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        ensure!(
            event.kind == Kind::Custom(38000),
            "Not a federation recommendation"
        );

        // TODO: make constant
        let federation_tag = SingleLetterTag::from_char('d').expect("Tag is valid");

        let event_id = event.id.to_bytes();

        let federation_id = event
            .tags()
            .iter()
            .find_map(|tag| {
                if tag.single_letter_tag() != Some(federation_tag) {
                    return None;
                }

                tag.as_vec().get(1)?.parse::<FederationId>().ok()
            })
            .context("No federation id tag found")?;

        let star_vote = extract_star_rating(&event.content);

        if let Some(star_vote) = star_vote {
            ensure!(star_vote <= 5, "Vote above 5 is invalid");
        }

        Ok(ParsedEvent {
            event_id,
            federation_id,
            star_vote,
        })
    }
}

async fn insert_parsed_event(
    dbtx: &deadpool_postgres::Transaction<'_>,
    parsed_event: ParsedEvent,
    event: Event,
) -> anyhow::Result<()> {
    debug!(
        "Inserting event {} for federation {}",
        hex::encode(parsed_event.event_id),
        parsed_event.federation_id
    );

    let now = chrono::Utc::now().naive_utc();
    dbtx.execute(
        // language=postgresql
        "INSERT INTO nostr_votes (event_id, federation_id, star_vote, event, fetch_time) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
        &[
            &parsed_event.event_id.to_vec(),
            &parsed_event.federation_id.consensus_encode_to_vec(),
            &parsed_event.star_vote.map(|vote| vote as i32),
            &serde_json::to_value(event).expect("can be serialized"),
            &now
        ],
    ).await?;

    Ok(())
}

fn extract_star_rating(comment: &str) -> Option<u8> {
    let re = Regex::new(r"^\[([0-9]+)/5]").expect("valid regex");
    let rating = re.captures(comment)?.get(1)?.as_str().parse::<u8>().ok()?;

    if (1..=5).contains(&rating) {
        Some(rating)
    } else {
        None
    }
}
