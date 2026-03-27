import { useQuery } from "@tanstack/react-query";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge.tsx";
import { Card } from "@/components/ui/card.tsx";
import { Separator } from "@/components/ui/separator.tsx";
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
	const feeDrag = Number(data?.totals.fee_drag ?? 0);

	const isPositive = unrealisedGain >= 0;

	return (
		<div className="@container/main container mx-auto space-y-6">
			<Card className="mt-6 flex flex-row p-6">
				<div className="flex flex-2 flex-col justify-between gap-1">
					<p className="font-bold text-muted-foreground text-xs uppercase tracking-widest">
						Total Value
					</p>
					<p className="font-extrabold text-5xl text-accent-foreground tabular-nums tracking-tight">
						{formatCurrency(totalValue)}
					</p>
				</div>
				<Separator orientation="vertical" />
				<div className="flex flex-1 flex-col justify-between gap-1">
					<div className="flex justify-between">
						<p className="font-bold text-muted-foreground text-xs uppercase tracking-widest">
							Performance
						</p>
						<Badge
							className={cn(
								"tabular-nums",
								isPositive
									? "bg-green-800 text-green-50"
									: "bg-red-800 text-red-50",
							)}
						>
							{isPositive ? <ArrowUpRight /> : <ArrowDownRight />}
							{formatPercentage(percentageGain)}
						</Badge>
					</div>
					<div className={cn("flex flex-col gap-1")}>
						<p
							className={cn(
								"font-extrabold text-3xl text-accent-foreground tabular-nums tracking-tight",
								isPositive ? "text-green-800" : "text-red-800",
							)}
						>
							{formatCurrency(unrealisedGain)}
						</p>
					</div>
				</div>
				<Separator orientation="vertical" />
				<div className="flex flex-1 flex-col justify-between gap-1">
					<div className="flex justify-between">
						<p className="font-bold text-muted-foreground text-xs uppercase tracking-widest">
							Fees
						</p>
						<Badge className="tabular-nums" variant="secondary">
							{formatPercentage(feeDrag)}
						</Badge>
					</div>
					<div className={cn("flex flex-col gap-1")}>
						<p
							className={cn(
								"font-extrabold text-3xl text-secondary-foreground tabular-nums tracking-tight",
							)}
						>
							{formatCurrency(totalFees)}
						</p>
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
					}}
				/>
			) : (
				"Loading"
			)}
		</div>
	);
};

export default HoldingsPage;
