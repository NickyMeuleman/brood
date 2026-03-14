import { useQuery } from "@tanstack/react-query";
import type { SortingState, VisibilityState } from "@tanstack/react-table";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge.tsx";
import {
	Card,
	CardAction,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card.tsx";
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldLabel,
} from "@/components/ui/field.tsx";
import { Switch } from "@/components/ui/switch.tsx";
import { cn, formatCurrency, formatPercentage } from "@/lib/utils.ts";
import { commands } from "../../bindings.ts";
import { createColumns } from "./columns.tsx";
import { DataTable } from "./data-table.tsx";

// Derive the initial column visibility from the column definitions once.
// `createColumns` always produces the same column shape regardless of the
// `includeFees` flag — only accessor functions differ — so reading
// hideByDefault off a single call is sufficient and stable across toggles.
function initialColumnVisibility(): VisibilityState {
	return createColumns(true).reduce((acc, col) => {
		const id = col.id ?? (col as any).accessorKey;
		if (id && col.meta?.hideByDefault) {
			acc[id] = false;
		}
		return acc;
	}, {} as VisibilityState);
}

const HoldingsPage = () => {
	const [includeFees, setIncludeFees] = useState(true);

	const [sorting, setSorting] = useState<SortingState>([]);
	const [columnVisibility, setColumnVisibility] = useState<VisibilityState>(
		initialColumnVisibility,
	);

	const { data } = useQuery({
		queryKey: ["holdings"],
		queryFn: async () => {
			const res = await commands.getHoldings();
			if (res.status === "error") throw new Error("oops");
			return res.data;
		},
	});

	const columns = useMemo(() => createColumns(includeFees), [includeFees]);
	// Workaround for tanstack table using stale cache values
	// Memoize the data array so it gets a new reference when fees toggle
	// biome-ignore lint/correctness/useExhaustiveDependencies: force update on fee change
	const tableData = useMemo(
		() => (data?.holdings ? [...data.holdings] : []),
		[data?.holdings, includeFees],
	);

	const total_eur = Number(data?.totals.market_value_eur) ?? 0;

	const unrealisedGain = includeFees
		? (Number(data?.totals.unrealised_gain_with_fees_eur) ?? 0)
		: (Number(data?.totals.unrealised_gain_eur) ?? 0);
	const percentageGain = includeFees
		? (Number(data?.totals.percentage_gain_with_fees) ?? 0)
		: (Number(data?.totals.percentage_gain) ?? 0);

	return (
		<div className="@container/main container mx-auto py-10">
			<div className="grid @5xl/main:grid-cols-4 @xl/main:grid-cols-2 grid-cols-1 gap-4 *:data-[slot=card]:shadow-xs">
				<Card className="@container/card">
					<CardHeader>
						<CardDescription>Total</CardDescription>
						<CardTitle className="font-semibold @[250px]/card:text-3xl text-2xl tabular-nums">
							{formatCurrency(total_eur)}
						</CardTitle>
						<CardAction></CardAction>
					</CardHeader>
					<CardFooter className="flex-col items-start gap-1.5 text-sm">
						<div className="line-clamp-1 flex gap-2 font-medium">
							Profit/Loss
						</div>
						<div
							className={cn(
								unrealisedGain > 0 ? "text-green-600" : "text-red-600",
								"line-clamp-1 flex items-center font-medium",
							)}
						>
							<Badge
								variant="outline"
								className={cn(
									unrealisedGain > 0
										? "bg-green-200/60 text-green-800"
										: "bg-red-200/60 text-red-800",
									"mr-2",
								)}
							>
								{unrealisedGain > 0 ? (
									<ArrowUpRight className="size-4" />
								) : (
									<ArrowDownRight className="size-4" />
								)}
								{formatPercentage(percentageGain)}
							</Badge>
							{formatCurrency(unrealisedGain)}
						</div>
					</CardFooter>
				</Card>
			</div>
			<div className="flex justify-end">
				<Field orientation="horizontal" className="max-w-sm p-5">
					<FieldContent>
						<FieldLabel htmlFor="include-fees-toggle">Include Fees</FieldLabel>
						<FieldDescription>
							Fees affect cost basis calculations
						</FieldDescription>
					</FieldContent>
					<Switch
						id="include-fees-toggle"
						checked={includeFees}
						onCheckedChange={(checked) => setIncludeFees(checked)}
					/>
				</Field>
			</div>
			{data ? (
				<DataTable
					columns={columns}
					data={tableData}
					meta={{ totals: data.totals }}
					sorting={sorting}
					onSortingChange={setSorting}
					columnVisibility={columnVisibility}
					onColumnVisibilityChange={setColumnVisibility}
				/>
			) : (
				"Loading"
			)}
		</div>
	);
};

export default HoldingsPage;
