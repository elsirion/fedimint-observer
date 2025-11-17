import { useMemo } from 'react';
import ReactEChartsCore from 'echarts-for-react/lib/core';
import * as echarts from 'echarts/core';
import { BarChart } from 'echarts/charts';
import { GridComponent, TooltipComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';

// Register only the components we need
echarts.use([
  BarChart,
  GridComponent,
  TooltipComponent,
  CanvasRenderer,
]);

// Small, local types for ECharts callback params to avoid using `any`
type TooltipParamItem = {
  axisValue?: string;
  value?: number;
  seriesName?: string;
  dataIndex?: number;
};
type ItemParam = { dataIndex: number };

interface MiniChartProps {
  data: number[];
  color: string;
  label: string;
  value: string;
  dates?: string[];
  formatValue?: (value: number) => string;
}

interface CombinedMiniChartProps {
  transactionData: number[];
  volumeData: number[];
  transactionValue: string;
  volumeValue: string;
  dates?: string[];
  formatTransaction?: (value: number) => string;
  formatVolume?: (value: number) => string;
  maxTransaction?: number;  // global max for consistent scale across all charts
  maxVolume?: number;        // global max for consistent scale across all charts
}

export function MiniChart({ data, color, label, value, dates, formatValue }: MiniChartProps) {
  const chartOption = useMemo(() => ({
    grid: {
      left: 0,
      right: 0,
      top: 0,
      bottom: 0,
    },
    xAxis: {
      type: 'category',
      show: false,
      data: dates || data.map((_, i) => `Day ${i + 1}`),
    },
    yAxis: {
      type: 'value',
      show: false,
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: '#1f2937',
      borderColor: '#374151',
      textStyle: { 
        color: '#fff', 
        fontSize: 11 
      },
      formatter: (params: unknown) => {
        const p = params as TooltipParamItem[];
        if (!p || p.length === 0) return '';
        const param = p[0];
        const dateStr = param.axisValue || '';
        const val = param.value;
        const formattedValue = formatValue ? formatValue(val as number) : (val !== undefined ? val.toString() : '');
        return `<div style="font-size: 11px;">
          <div style="color: #9ca3af; margin-bottom: 2px;">${dateStr}</div>
          <div style="font-weight: 600;">${label}: ${formattedValue}</div>
        </div>`;
      },
      axisPointer: {
        type: 'line',
        lineStyle: {
          color: color,
          width: 1,
          type: 'solid',
        },
      },
    },
    series: [
      {
        type: 'bar',
        data: data,
        itemStyle: {
          color: color,
          borderRadius: [2, 2, 0, 0],
        },
        barWidth: '60%',
      },
    ],
  }), [data, color, dates, formatValue, label]);

  return (
    <div className="flex flex-col sm:flex-row sm:items-center gap-2">
      <div className="flex-1">
        <div className="text-xs text-gray-600 dark:text-gray-400">{label}</div>
        <div className="text-sm font-medium text-gray-900 dark:text-white">{value}</div>
      </div>
      <div className="w-24 h-12 sm:w-32 sm:h-12">
        <ReactEChartsCore
          echarts={echarts}
          option={chartOption}
          notMerge={true}
          lazyUpdate={true}
          style={{ height: '100%', width: '100%' }}
          opts={{ renderer: 'canvas' }}
        />
      </div>
    </div>
  );
}

export function CombinedMiniChart({ 
  transactionData, 
  volumeData, 
  transactionValue, 
  volumeValue, 
  dates,
  formatTransaction,
  formatVolume,
  maxTransaction,
  maxVolume
}: CombinedMiniChartProps) {
  const chartOption = useMemo(() => ({
    grid: {
      left: 0,
      right: 0,
      top: 0,
      bottom: 0,
    },
    xAxis: {
      type: 'category',
      show: false,
      data: dates || transactionData.map((_, i) => `Day ${i + 1}`),
    },
    yAxis: [
      {
        type: 'value',
        show: false,
        min: 0,
        max: maxTransaction,  
        minInterval: 1,
      },
      {
        type: 'value',
        show: false,
        min: 0,
        max: maxVolume,       
      }
    ],
    tooltip: {
      trigger: 'axis',
      backgroundColor: '#1f2937',
      borderColor: '#374151',
      textStyle: { 
        color: '#fff', 
        fontSize: 11 
      },
      formatter: (params: unknown) => {
        const p = params as TooltipParamItem[];
        if (!p || p.length === 0) return '';
        const dateStr = p[0].axisValue || '';
        let result = `<div style="font-size: 11px;">
          <div style="color: #9ca3af; margin-bottom: 4px;">${dateStr}</div>`;
        
        p.forEach((param: TooltipParamItem) => {
          // Get the actual original value (not the adjusted one for display)
          let actualValue;
          const idx = param.dataIndex ?? 0;
          if (param.seriesName === 'Transactions') {
            actualValue = transactionData[idx];
          } else {
            actualValue = volumeData[idx];
          }
          
          let formattedValue = '';
          if (param.seriesName === 'Transactions') {
            formattedValue = formatTransaction ? formatTransaction(actualValue) : actualValue.toString();
          } else {
            formattedValue = formatVolume ? formatVolume(actualValue) : actualValue.toString();
          }
          result += `<div style="font-weight: 600; color: ${param.seriesName === 'Transactions' ? '#3b82f6' : '#10b981'};">
            ${param.seriesName}: ${formattedValue}
          </div>`;
        });
        
        result += '</div>';
        return result;
      },
      axisPointer: {
        type: 'shadow',
      },
    },
    series: [
      {
        name: 'Transactions',
        type: 'bar',
        data: transactionData.map((val) => {
          if (val === 0) {
            const hasData = transactionData.some(v => v > 0);
            if (hasData) {
              const maxVal = Math.max(...transactionData);
              return maxVal * 0.02; // 2% of max value as minimum visible bar
            } else {
              // All values are zero, use a small fixed value
              return 0.1;
            }
          }
          return val;
        }),
        yAxisIndex: 0,
        itemStyle: {
          color: (params: unknown) => {
            const p = params as ItemParam;
            // Make zero values more transparent
            return transactionData[p.dataIndex] === 0 ? '#3b82f633' : '#3b82f6';
          },
          borderRadius: [2, 2, 0, 0],
        },
        barGap: '20%',
        barWidth: '30%',
      },
      {
        name: 'Volume',
        type: 'bar',
        data: volumeData.map((val) => {
          if (val === 0) {
            const hasData = volumeData.some(v => v > 0);
            if (hasData) {
              const maxVal = Math.max(...volumeData);
              return maxVal * 0.02; // 2% of max value as minimum visible bar
            } else {
              // All values are zero, use a small fixed value
              return 0.0000001;
            }
          }
          return val;
        }),
        yAxisIndex: 1,
        itemStyle: {
          color: (params: unknown) => {
            const p = params as ItemParam;
            // Make zero values more transparent
            return volumeData[p.dataIndex] === 0 ? '#10b98133' : '#10b981';
          },
          borderRadius: [2, 2, 0, 0],
        },
        barWidth: '30%',
      },
    ],
  }), [transactionData, volumeData, dates, formatTransaction, formatVolume, maxTransaction, maxVolume]);

  return (
    <div className="flex flex-col gap-2 md:gap-3">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <div className="flex items-center gap-1 md:gap-1.5 mb-0.5 md:mb-1">
            <div className="w-2 h-2 md:w-2.5 xl:w-3 md:h-2.5 xl:h-3 rounded-sm flex-shrink-0" style={{ backgroundColor: '#3b82f6' }}></div>
            <div className="text-[9px] md:text-[7px] xl:text-xs text-gray-600 dark:text-gray-400 truncate">Transactions</div>
          </div>
          <div className="text-[11px] md:text-xxs xl:text-sm font-medium text-gray-900 dark:text-white truncate">{transactionValue}</div>
        </div>
        <div>
          <div className="flex items-center gap-1 md:gap-1.5 mb-0.5 md:mb-1">
            <div className="w-2 h-2 md:w-2.5 xl:w-3 md:h-2.5 xl:h-3 rounded-sm flex-shrink-0" style={{ backgroundColor: '#10b981' }}></div>
            <div className="text-[9px] md:text-[10px] xl:text-xs text-gray-600 dark:text-gray-400 truncate">Volume</div>
          </div>
          <div className="text-[11px] md:text-xxs xl:text-sm font-medium text-gray-900 dark:text-white truncate">{volumeValue}</div>
        </div>
      </div>
      <div className="w-full h-10 md:h-14 xl:h-16">
        <ReactEChartsCore
          echarts={echarts}
          option={chartOption}
          notMerge={true}
          lazyUpdate={true}
          style={{ height: '100%', width: '100%' }}
          opts={{ renderer: 'canvas' }}
        />
      </div>
    </div>
  );
}
