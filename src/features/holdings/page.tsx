import { useQuery } from "@tanstack/react-query";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge.tsx";
import { Card } from "@/components/ui/card.tsx";
import { Separator } from "@/components/ui/separator.tsx";
import {
	cn,
	formatCurrency,
	formatPercentage,
	PERIOD_LABEL,
	PERIODS,
} from "@/lib/utils.ts";
import { commands, type Period } from "../../bindings.ts";
import { columns, normalizeHolding } from "./columns.tsx";
import { DataTable } from "./data-table.tsx";

const HoldingsPage = () => {
	const [includeFees, setIncludeFees] = useState(true);
	const [displayInEur, setDisplayInEur] = useState(false);
	const [period, setPeriod] = useState<Period>("AllTime");

	const { data } = useQuery({
		queryKey: ["holdings", period],
		queryFn: async () => {
			const res = await commands.getHoldings("AllTime");
			if (res.status === "error") throw new Error("oops");
			return res.data;
		},
	});

	const tableData = useMemo(
		() =>
			data?.holdings.map((h) =>
				normalizeHolding(h, includeFees, displayInEur),
			) ?? [],
		[data?.holdings, includeFees, displayInEur],
	);

	const totalValue = Number(data?.totals.market_value_eur ?? 0);
	const unrealisedGain = includeFees
		? Number(data?.totals.unrealised_gain_with_fees_eur ?? 0)
		: Number(data?.totals.unrealised_gain_eur ?? 0);
	const percentageGain = includeFees
		? Number(data?.totals.pct_gain_with_fees ?? 0)
		: Number(data?.totals.pct_gain ?? 0);
	const totalFees = Number(data?.totals.total_fees_eur ?? 0);
	const feeDrag = Number(data?.totals.fee_drag ?? 0);

	const isPositive = unrealisedGain >= 0;

	return (
		<div className="@container/main container mx-auto space-y-6 tabular-nums">
			<Card className="flex flex-row gap-4 p-4">
				{/* <div className="grid grid-cols-3 gap-3"> */}
				<div className="flex flex-1 flex-col justify-between p-2">
					{/* <Card className="row-span-2 flex flex-col justify-between gap-1 rounded-xl p-6 text-accent-foreground"> */}
					{/* <div className="row-span-2 flex flex-col justify-between gap-1 rounded-xl bg-accent p-6 text-accent-foreground"> */}
					<p className="font-bold text-xs uppercase tracking-widest">
						Total Value
					</p>
					<p className="font-extrabold text-accent-foreground text-xl tracking-tight md:text-3xl xl:text-6xl">
						{formatCurrency(totalValue)}
					</p>
					{/* </div> */}
					{/* </Card> */}
				</div>
				<Separator orientation="vertical" />
				<div className="flex flex-1 gap-4">
					<div className="flex flex-1 flex-col gap-2">
						<div className="p-2">
							{/* <Card */}
							{/* 	className={cn( */}
							{/* 		"flex flex-col justify-between gap-1 rounded-xl p-6", */}
							{/* 		isPositive ? "text-green-800" : "text-red-800", */}
							{/* 	)} */}
							{/* > */}
							{/* <div */}
							{/* 	className={cn( */}
							{/* 		"flex flex-col justify-between gap-1 rounded-xl p-6", */}
							{/* 		isPositive */}
							{/* 			? "bg-green-50 text-green-800" */}
							{/* 			: "bg-red-50 text-red-800", */}
							{/* 	)} */}
							{/* > */}
							<div className="flex justify-between">
								<p className="font-bold text-xs uppercase tracking-widest">
									Performance
								</p>
								<Badge
									variant="outline"
									className={cn(
										isPositive
											? "border-green-800 bg-green-50 text-green-800"
											: "border-red-800 bg-red-50 text-red-800",
									)}
								>
									{/* <Badge */}
									{/* 	className={cn( */}
									{/* 		isPositive */}
									{/* 			? "bg-green-800 text-green-50" */}
									{/* 			: "bg-red-800 text-red-50", */}
									{/* 	)} */}
									{/* > */}
									{isPositive ? <ArrowUpRight /> : <ArrowDownRight />}
									{formatPercentage(percentageGain)}
								</Badge>
							</div>
							<div className="flex flex-col gap-1">
								<p
									className={cn(
										"font-extrabold text-3xl tracking-tight",
										isPositive ? "text-green-800" : "text-red-800",
									)}
								>
									{formatCurrency(unrealisedGain)}
								</p>
							</div>
							{/* </div> */}
							{/* </Card> */}
						</div>
						<Separator />
						<div className="p-2">
							{/* <Card className="flex flex-col justify-between gap-1 rounded-xl p-6 text-secondary-foreground"> */}
							{/* <div className="flex flex-col justify-between gap-1 rounded-xl bg-secondary p-6 text-secondary-foreground"> */}
							<div className="flex justify-between">
								<p className="font-bold text-xs uppercase tracking-widest">
									Fees
								</p>
								<Badge
									variant="outline"
									className="border-secondary-foreground bg-secondary text-secondary-foreground"
								>
									{/* <Badge className="bg-secondary-foreground text-secondary"> */}
									{formatPercentage(feeDrag)}
								</Badge>
							</div>
							<div className="flex flex-col gap-1">
								<p className="font-extrabold text-3xl text-secondary-foreground tracking-tight">
									{formatCurrency(totalFees)}
								</p>
							</div>
							{/* </div> */}
							{/* </Card> */}
						</div>
					</div>
					<Separator orientation="vertical" />
					<div className="flex flex-1 flex-col gap-2">
						<div className="p-2">
							{/* <Card */}
							{/* 	className={cn( */}
							{/* 		"flex flex-col justify-between gap-1 rounded-xl p-6", */}
							{/* 		!isPositive ? "text-green-800" : "text-red-800", */}
							{/* 	)} */}
							{/* > */}

							{/* <div */}
							{/* 	className={cn( */}
							{/* 		"flex flex-col justify-between gap-1 rounded-xl p-6", */}
							{/* 		!isPositive */}
							{/* 			? "bg-green-50 text-green-800" */}
							{/* 			: "bg-red-50 text-red-800", */}
							{/* 	)} */}
							{/* > */}
							<div className="flex justify-between">
								<p className="font-bold text-xs uppercase tracking-widest">
									Performance ({PERIOD_LABEL[period]})
								</p>
								<Badge
									variant="outline"
									className={cn(
										!isPositive
											? "border-green-800 bg-green-50 text-green-800"
											: "border-red-800 bg-red-50 text-red-800",
									)}
								>
									{/* <Badge */}
									{/* 	className={cn( */}
									{/* 		!isPositive */}
									{/* 			? "bg-green-800 text-green-50" */}
									{/* 			: "bg-red-800 text-red-50", */}
									{/* 	)} */}
									{/* > */}
									{!isPositive ? <ArrowUpRight /> : <ArrowDownRight />}
									{formatPercentage(0)}
								</Badge>
							</div>
							<div className="flex flex-col gap-1">
								<p
									className={cn(
										"font-extrabold text-3xl tracking-tight",
										!isPositive ? "text-green-800" : "text-red-800",
									)}
								>
									{formatCurrency(0)}
								</p>
							</div>
							{/* </div> */}
							{/* </Card> */}
						</div>
						<Separator />
						<div className="p-2">
							{/* <Card className="flex flex-col justify-between gap-1 rounded-xl p-6 text-secondary-foreground"> */}
							{/* <div className="flex flex-col justify-between gap-1 rounded-xl bg-secondary p-6 text-secondary-foreground"> */}
							<div className="flex justify-between">
								<p className="font-bold text-xs uppercase tracking-widest">
									Fees ({PERIOD_LABEL[period]})
								</p>
								<Badge
									variant="outline"
									className="border-secondary-foreground bg-secondary text-secondary-foreground"
								>
									{/* <Badge className="bg-secondary-foreground text-secondary"> */}
									{formatPercentage(0)}
								</Badge>
							</div>
							<div className="flex flex-col gap-1">
								<p className="font-extrabold text-3xl text-secondary-foreground tracking-tight">
									{formatCurrency(0)}
								</p>
							</div>
							{/* </Card> */}
							{/* </div> */}
						</div>
						{/* </div> */}
					</div>
				</div>
			</Card>
			{data ? (
				<DataTable
					columns={columns}
					data={tableData}
					meta={{ totals: data.totals }}
					display={{
						includeFees,
						onIncludeFeesChange: setIncludeFees,
						displayInEur,
						onDisplayInEurChange: setDisplayInEur,
						periods: PERIODS,
						period,
						onPeriodChange: setPeriod,
					}}
				/>
			) : (
				"Loading"
			)}
		</div>
	);
};

export default HoldingsPage;
