import { useQuery } from "@tanstack/react-query";
import {
	AreaChart,
	ArrowDownRight,
	ArrowUpRight,
	BanknoteArrowDown,
	TrendingDown,
	TrendingUp,
	TrendingUpDown,
	Wallet,
} from "lucide-react";
import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge.tsx";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card.tsx";
import { cn, formatCurrency, formatPercentage } from "@/lib/utils.ts";
import { commands } from "../../bindings.ts";
import { columns, normalizeHolding } from "./columns.tsx";
import { DataTable } from "./data-table.tsx";

const HoldingsPage = () => {
	const [includeFees, setIncludeFees] = useState(false);
	const [displayInEur, setDisplayInEur] = useState(false);

	const { data } = useQuery({
		queryKey: ["holdings"],
		queryFn: async () => {
			const res = await commands.getHoldings();
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
	const totalCost = includeFees
		? Number(data?.totals.total_with_fees_eur ?? 0)
		: Number(data?.totals.total_cost_eur ?? 0);
	const feeDrag = Number(data?.totals.fee_drag);

	const isPositive = unrealisedGain >= 0;

	return (
		<div className="@container/main container mx-auto space-y-10">
			{/* Summary cards — always EUR */}
			<div className="grid @xl/main:grid-cols-3 grid-cols-1 gap-4 *:data-[slot=card]:shadow-xs">
				<Card>
					<CardHeader>
						<CardDescription className="flex items-center gap-1.5">
							<Wallet className="h-3.5 w-3.5" />
							Portfolio Value
						</CardDescription>
						<CardTitle className="font-semibold text-3xl tabular-nums">
							{formatCurrency(totalValue, "EUR")}
						</CardTitle>
					</CardHeader>
					<CardFooter className="flex gap-1.5 text-muted-foreground text-sm">
						<span>Paid {formatCurrency(totalCost, "EUR")}</span>
						<span className="text-muted-foreground/70 text-xs">
							{`(fees ${includeFees ? "included" : "excluded"})`}
						</span>
					</CardFooter>
				</Card>

				<Card>
					<CardHeader>
						<CardDescription className="flex items-center gap-1.5">
							<TrendingUpDown className="h-3.5 w-3.5" />
							Unrealised P&amp;L
						</CardDescription>
						<CardTitle
							className={cn(
								"font-semibold text-3xl tabular-nums",
								isPositive ? "text-green-600" : "text-red-600",
							)}
						>
							{formatCurrency(unrealisedGain, "EUR")}
						</CardTitle>
					</CardHeader>
					<CardFooter className="flex gap-1.5 text-sm">
						<Badge
							variant="outline"
							className={cn(
								isPositive
									? "border-green-800/60 bg-green-100/60 text-green-800 dark:bg-green-900/30 dark:text-green-400"
									: "border-red-800/60 bg-red-100/60 text-red-800 dark:bg-red-900/30 dark:text-red-400",
							)}
						>
							{isPositive ? <ArrowUpRight /> : <ArrowDownRight />}
							{formatPercentage(percentageGain)}
						</Badge>
						<span className="text-muted-foreground">all time</span>
						<span className="text-muted-foreground/70 text-xs">
							{`(fees ${includeFees ? "included" : "excluded"})`}
						</span>
					</CardFooter>
				</Card>

				<Card>
					<CardHeader>
						<CardDescription className="flex items-center gap-1.5">
							<BanknoteArrowDown className="h-3.5 w-3.5" />
							Fees Paid
						</CardDescription>
						<CardTitle className="font-semibold text-3xl text-muted-foreground tabular-nums">
							{formatCurrency(totalFees, "EUR")}
						</CardTitle>
					</CardHeader>
					<CardFooter className="text-muted-foreground text-sm">
						{totalCost > 0
							? `${formatPercentage(feeDrag)} of invested capital`
							: "—"}
					</CardFooter>
				</Card>
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
					}}
				/>
			) : (
				"Loading"
			)}
		</div>
	);
};

export default HoldingsPage;
