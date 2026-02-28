import React from "react";
import { Area, AreaChart, CartesianGrid, XAxis } from "recharts";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import {
	type ChartConfig,
	ChartContainer,
	ChartLegend,
	ChartLegendContent,
	ChartTooltip,
	ChartTooltipContent,
} from "@/components/ui/chart";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import chartData from "@/features/chart/data.ts";

const selectItems = [
	{
		value: "90d",
		label: "Last 3 months",
		days: 90,
	},
	{
		value: "30d",
		label: "Last 30 days",
		days: 30,
	},
	{
		value: "7d",
		label: "Last 7 days",
		days: 7,
	},
] as const;

const chartConfig = {
	visitors: {
		label: "Visitors",
	},
	desktop: {
		label: "Desktop",
		color: "var(--chart-1)",
	},
	mobile: {
		label: "Mobile",
		color: "var(--chart-2)",
	},
} satisfies ChartConfig;

const ChartPage = () => {
	const [timeRange, setTimeRange] = React.useState<string>(
		selectItems[0].value,
	);
	const activeItem =
		selectItems.find((item) => item.value === timeRange) || selectItems[0];

	const filteredData = chartData.filter((item) => {
		const date = new Date(item.date);
		const referenceDate = new Date("2024-06-30");
		const startDate = new Date(referenceDate);
		startDate.setDate(startDate.getDate() - activeItem.days);
		return date >= startDate;
	});

	return (
		<div className="m-auto max-w-3/4 p-5">
			<Card className="pt-0">
				<CardHeader className="flex items-center gap-2 space-y-0 border-b py-5 sm:flex-row">
					<div className="grid flex-1 gap-1">
						<CardTitle>Area Chart - Interactive</CardTitle>
						<CardDescription>
							Showing total visitors for the last 3 months
						</CardDescription>
					</div>
					<Select
						items={selectItems}
						value={timeRange}
						onValueChange={(value) => setTimeRange(value || "")}
					>
						<SelectTrigger
							className="hidden w-40 rounded-lg sm:ml-auto sm:flex"
							aria-label="Select a value"
						>
							<SelectValue />
						</SelectTrigger>
						<SelectContent className="rounded-xl">
							<SelectGroup>
								<SelectLabel>Tijdspanne</SelectLabel>
								{selectItems.map((item) => (
									<SelectItem
										key={item.value}
										value={item.value}
										className="rounded-lg"
									>
										{item.label}
									</SelectItem>
								))}
							</SelectGroup>
						</SelectContent>
					</Select>
				</CardHeader>
				<CardContent className="px-2 pt-4 sm:px-6 sm:pt-6">
					<ChartContainer
						config={chartConfig}
						className="aspect-auto h-62.5 w-full"
					>
						<AreaChart data={filteredData}>
							<defs>
								<linearGradient id="fillDesktop" x1="0" y1="0" x2="0" y2="1">
									<stop
										offset="5%"
										stopColor="var(--color-desktop)"
										stopOpacity={0.8}
									/>
									<stop
										offset="95%"
										stopColor="var(--color-desktop)"
										stopOpacity={0.1}
									/>
								</linearGradient>
								<linearGradient id="fillMobile" x1="0" y1="0" x2="0" y2="1">
									<stop
										offset="5%"
										stopColor="var(--color-mobile)"
										stopOpacity={0.8}
									/>
									<stop
										offset="95%"
										stopColor="var(--color-mobile)"
										stopOpacity={0.1}
									/>
								</linearGradient>
							</defs>
							<CartesianGrid vertical={false} />
							<XAxis
								dataKey="date"
								tickLine={false}
								axisLine={false}
								tickMargin={8}
								minTickGap={32}
								tickFormatter={(value) => {
									const date = new Date(value);
									return date.toLocaleDateString("nl-BE", {
										month: "short",
										day: "numeric",
									});
								}}
							/>
							<ChartTooltip
								cursor={false}
								content={
									<ChartTooltipContent
										labelFormatter={(value) => {
											return new Date(value).toLocaleDateString("nl-BE", {
												month: "short",
												day: "numeric",
											});
										}}
										indicator="dot"
									/>
								}
							/>
							<Area
								dataKey="mobile"
								type="natural"
								fill="url(#fillMobile)"
								stroke="var(--color-mobile)"
								stackId="a"
							/>
							<Area
								dataKey="desktop"
								type="natural"
								fill="url(#fillDesktop)"
								stroke="var(--color-desktop)"
								stackId="a"
							/>
							<ChartLegend content={<ChartLegendContent />} />
						</AreaChart>
					</ChartContainer>
				</CardContent>
			</Card>
		</div>
	);
};

export default ChartPage;
