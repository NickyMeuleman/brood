import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import { commands, type Period } from "@/bindings";
import { Card, CardContent } from "@/components/ui/card";
import {
	type ChartConfig,
	ChartContainer,
	ChartTooltip,
	ChartTooltipContent,
} from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import { useDateFormatters } from "@/hooks/use-date-formatters";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { formatCurrency } from "@/lib/utils";
import { useUIStore } from "@/stores/ui";

const PERIOD_TICK_COUNTS: Record<Period, number> = {
	FiveDays: 5, // 1 label per day
	OneMonth: 5, // Roughly 1 label per week
	SixMonths: 6, // 1 label per month
	Ytd: 6, // Well-spaced months
	OneYear: 6, // 1 label every 2 months
	FiveYears: 5, // 1 label per year
	AllTime: 5, // Scaled representation
};

function getXAxisFormatter(
	period: Period,
	formatters: ReturnType<typeof useDateFormatters>,
): (value: string) => string {
	const map: Record<Period, Intl.DateTimeFormat> = {
		FiveDays: formatters.weekday,
		OneMonth: formatters.dayMonth,
		SixMonths: formatters.dayMonth,
		OneYear: formatters.dayMonth,
		Ytd: formatters.dayMonth,
		FiveYears: formatters.monthYear,
		AllTime: formatters.monthYear,
	};
	return (value: string) => {
		const date = new Date(`${value}T00:00:00`);
		const formatter = map[period] ?? formatters.monthYear;
		return formatter.format(date);
	};
}

export function PortfolioChart() {
	const { period, includeFees } = useUIStore();
	const formatters = useDateFormatters();

	const { data, isLoading } = useQuery({
		queryKey: queryKeys.portfolioHistoryByPeriod(period),
		queryFn: async () => {
			const res = await commands.getPortfolioHistory(period);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
	});

	const chartData = useMemo(() => {
		if (!data?.points) return [];
		return data.points.map((pt) => ({
			date: pt.date,
			value: Number(pt.totals.value_eur),
			invested: includeFees
				? Number(pt.totals.invested_eur.net)
				: Number(pt.totals.invested_eur.gross),
		}));
	}, [data, includeFees]);

	const chartConfig = {
		value: {
			label: "Value",
			color: "var(--chart-1)",
		},
		invested: {
			label: "Invested",
			color: "var(--chart-5)",
		},
	} satisfies ChartConfig;

	const xTicks = useMemo(() => {
		if (!chartData?.length) return [];

		const count = PERIOD_TICK_COUNTS[period] ?? chartData.length;
		const ticks = [];
		for (let i = 0; i < count; i++) {
			const idx = Math.round((i * (chartData.length - 1)) / (count - 1));
			const date = chartData[idx]?.date;
			if (date) {
				ticks.push(date);
			}
		}

		return [...new Set(ticks)];
	}, [chartData, period]);

	if (isLoading) {
		return <Skeleton className="h-64 w-full rounded-xl" />;
	}

	return (
		<Card>
			<CardContent className="px-2 pt-4 pb-2 sm:px-6 sm:pt-6">
				<ChartContainer config={chartConfig} className="h-64 min-h-32 w-full">
					<AreaChart
						data={chartData}
						margin={{ top: 4, right: 20, bottom: 0, left: 20 }}
					>
						<defs>
							<linearGradient
								id="portfolioValueFill"
								x1="0"
								y1="0"
								x2="0"
								y2="1"
							>
								<stop
									offset="5%"
									stopColor="var(--color-value)"
									stopOpacity={0.25}
								/>
								<stop
									offset="95%"
									stopColor="var(--color-value)"
									stopOpacity={0.02}
								/>
							</linearGradient>
						</defs>

						<CartesianGrid vertical={false} />

						<XAxis
							dataKey="date"
							ticks={xTicks}
							tickLine={false}
							axisLine={false}
							tickMargin={12}
							minTickGap={40}
							interval="preserveStartEnd"
							tickFormatter={getXAxisFormatter(period, formatters)}
						/>

						<YAxis hide domain={[0, (max: number) => max * 1.05]} />

						<ChartTooltip
							cursor={{ stroke: "var(--border)", strokeWidth: 1 }}
							content={
								<ChartTooltipContent
									labelFormatter={(value) =>
										formatters.fulldate.format(new Date(`${value}T00:00:00`))
									}
									formatter={(value, name, item) => (
										<>
											<div className="flex items-center gap-1.5">
												<div
													className="h-2 w-2 shrink-0 rounded-[2px]"
													style={{ backgroundColor: item.color }}
												/>
												<span className="text-muted-foreground">
													{item.dataKey === "invested"
														? `Invested${includeFees ? " (incl. fees)" : ""}`
														: chartConfig[
																item.dataKey as keyof typeof chartConfig
															]?.label || name}
												</span>
											</div>
											<span className="ml-auto pl-6 font-medium font-mono text-foreground tabular-nums">
												{formatCurrency(Number(value))}
											</span>
										</>
									)}
								/>
							}
						/>

						<Area
							type="monotone"
							dataKey="value"
							stroke="var(--color-value)"
							strokeWidth={2}
							fill="url(#portfolioValueFill)"
							dot={false}
							activeDot={{ r: 3 }}
						/>
						<Area
							type="stepAfter"
							dataKey="invested"
							stroke="var(--color-invested)"
							strokeWidth={1.5}
							fill="none"
							strokeDasharray="4 4"
							dot={false}
							activeDot={{ r: 3 }}
						/>
					</AreaChart>
				</ChartContainer>
			</CardContent>
		</Card>
	);
}
