import { useQuery } from "@tanstack/react-query";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import { useMemo } from "react";
import { Badge } from "@/components/ui/badge.tsx";
import { getErrorMessage } from "@/lib/errors.ts";
import { queryKeys } from "@/lib/queryKeys.ts";
import {
	cn,
	formatCurrency,
	formatPercentage,
	PERIOD_LABEL,
} from "@/lib/utils.ts";
import { useUIStore } from "@/stores/ui.ts";
import { commands } from "../../bindings.ts";
import { columns } from "./columns.tsx";
import { DataTable } from "./data-table.tsx";
import { normalizeHolding } from "./lib.ts";
import { PortfolioChart } from "./PortfolioChart.tsx";

const HoldingsPage = () => {
	const { includeFees, displayInEur, period } = useUIStore();

	const { data } = useQuery({
		queryKey: queryKeys.holdingsByPeriod(period),
		queryFn: async () => {
			const res = await commands.getHoldings(period);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
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

	const totalValue = Number(data?.totals.value ?? 0);
	const gain = includeFees
		? Number(data?.totals.all_time.net.gain ?? 0)
		: Number(data?.totals.all_time.gross.gain ?? 0);
	const periodGain = includeFees
		? Number(data?.totals.period.net.gain ?? 0)
		: Number(data?.totals.period.gross.gain ?? 0);
	const percentageGain = includeFees
		? Number(data?.totals.all_time.net.pct_gain ?? 0)
		: Number(data?.totals.all_time.gross.pct_gain ?? 0);
	const periodPercentageGain = includeFees
		? Number(data?.totals.period.net.pct_gain ?? 0)
		: Number(data?.totals.period.gross.pct_gain ?? 0);
	const fees = Number(data?.totals.all_time.fees ?? 0);
	const periodFees = Number(data?.totals.period.fees ?? 0);
	const feeDrag = Number(data?.totals.all_time.fee_drag ?? 0);
	const periodFeeDrag = Number(data?.totals.period.fee_drag ?? 0);

	const isPositive = gain >= 0;
	const isPeriodPositive = periodGain >= 0;

	return (
		<div className="container mx-auto space-y-6 tabular-nums">
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
							? "bg-emerald-50 text-emerald-800"
							: "bg-rose-50 text-rose-800",
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
								isPositive ? "border-emerald-800" : "border-rose-800",
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
							? "bg-emerald-50 text-emerald-800"
							: "bg-rose-50 text-rose-800",
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
								isPeriodPositive ? "border-emerald-800" : "border-rose-800",
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
      <div>
        <PortfolioChart />
      </div>
			{data ? (
				<DataTable
					columns={columns}
					data={tableData}
					meta={{ totals: data.totals, period }}
				/>
			) : (
				"Loading"
			)}
		</div>
	);
};

export default HoldingsPage;
