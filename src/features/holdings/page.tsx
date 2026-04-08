import { useQuery } from "@tanstack/react-query";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge.tsx";
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
			const res = await commands.getHoldings(period);
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
	const gain = includeFees
		? Number(data?.totals.gain_with_fees_eur ?? 0)
		: Number(data?.totals.gain_eur ?? 0);
	const periodGain = includeFees
		? Number(data?.totals.period_gain_with_fees_eur ?? 0)
		: Number(data?.totals.period_gain_eur ?? 0);
	const percentageGain = includeFees
		? Number(data?.totals.pct_gain_with_fees ?? 0)
		: Number(data?.totals.pct_gain ?? 0);
	const periodPercentageGain = includeFees
		? Number(data?.totals.period_pct_gain_with_fees ?? 0)
		: Number(data?.totals.period_pct_gain ?? 0);
	const fees = Number(data?.totals.fees_eur ?? 0);
	const periodFees = Number(data?.totals.period_fees_eur ?? 0);
	const feeDrag = Number(data?.totals.fee_drag ?? 0);
	const periodFeeDrag = Number(data?.totals.period_fee_drag ?? 0);

	const isPositive = gain >= 0;
	const isPeriodPositive = periodGain >= 0;

	return (
		<div className="@container/main container mx-auto space-y-6 tabular-nums">
			<div className="grid grid-cols-3 gap-3">
				<div className="row-span-2 flex flex-col justify-between gap-1 rounded-xl bg-accent p-6 text-accent-foreground">
					<p className="font-bold text-xs uppercase tracking-widest">
						Total Value
					</p>
					<p className="font-extrabold text-accent-foreground text-xl tracking-tight md:text-3xl xl:text-6xl">
						{formatCurrency(totalValue)}
					</p>
				</div>
				<div
					className={cn(
						"flex flex-col justify-between gap-1 rounded-xl p-6",
						isPositive
							? "bg-green-50 text-green-800"
							: "bg-red-50 text-red-800",
					)}
				>
					<div className="flex justify-between">
						<p className="font-bold text-xs uppercase tracking-widest">
							Performance
						</p>
						<Badge
							variant="outline"
							className={cn(
								"bg-inherit text-inherit",
								isPositive ? "border-green-800" : "border-red-800",
							)}
						>
							{isPositive ? <ArrowUpRight /> : <ArrowDownRight />}
							{formatPercentage(percentageGain)}
						</Badge>
					</div>
					<p className="font-extrabold text-3xl tracking-tight">
						{formatCurrency(gain)}
					</p>
				</div>
				<div className="flex flex-col justify-between gap-1 rounded-xl bg-secondary p-6 text-secondary-foreground">
					<div className="flex justify-between">
						<p className="font-bold text-xs uppercase tracking-widest">Fees</p>
						<Badge
							variant="outline"
							className="border-secondary-foreground bg-secondary text-secondary-foreground"
						>
							{formatPercentage(feeDrag)}
						</Badge>
					</div>
					<p className="font-extrabold text-3xl text-secondary-foreground tracking-tight">
						{formatCurrency(fees)}
					</p>
				</div>
				<div
					className={cn(
						"flex flex-col justify-between gap-1 rounded-xl p-6",
						isPeriodPositive
							? "bg-green-50 text-green-800"
							: "bg-red-50 text-red-800",
					)}
				>
					<div className="flex justify-between">
						<p className="font-bold text-xs uppercase tracking-widest">
							Performance ({PERIOD_LABEL[period]})
						</p>
						<Badge
							variant="outline"
							className={cn(
								"bg-inherit text-inherit",
								isPeriodPositive ? "border-green-800" : "border-red-800",
							)}
						>
							{isPeriodPositive ? <ArrowUpRight /> : <ArrowDownRight />}
							{formatPercentage(periodPercentageGain)}
						</Badge>
					</div>
					<p className="font-extrabold text-3xl tracking-tight">
						{formatCurrency(periodGain)}
					</p>
				</div>
				<div className="flex flex-col justify-between gap-1 rounded-xl bg-secondary p-6 text-secondary-foreground">
					<div className="flex justify-between">
						<p className="font-bold text-xs uppercase tracking-widest">
							Fees ({PERIOD_LABEL[period]})
						</p>
						<Badge
							variant="outline"
							className="border-secondary-foreground bg-secondary text-secondary-foreground"
						>
							{formatPercentage(periodFeeDrag)}
						</Badge>
					</div>
					<p className="font-extrabold text-3xl text-secondary-foreground tracking-tight">
						{formatCurrency(periodFees)}
					</p>
				</div>
			</div>
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
