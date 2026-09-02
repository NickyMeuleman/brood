import type {
	ListingInfo,
	Pre2026CostBasisMethod,
	SellComputation,
} from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardAction,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import {
	Sheet,
	SheetContent,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
} from "@/components/ui/sheet";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { useDateFormatters } from "@/hooks/use-date-formatters";
import { formatCurrency, MIC_LABEL } from "@/lib/utils";

const METHOD_LABEL: Record<Pre2026CostBasisMethod, string> = {
	FOTOMOMENT: "Fotomoment (2025-12-31)",
	HISTORICAL_ELECTED: "Historical cost",
	FLOORED_AT_ZERO: "Historical cost, floored to zero",
};

export function SellTaxDetailSheet({
	computation,
	listings,
}: {
	computation: SellComputation;
	listings: ListingInfo[];
}) {
	const dateformatters = useDateFormatters();
	// TODO: be specific about listing (id instead of isin)
	const listing = listings.find((l) => l.isin === computation.isin);

	return (
		<Sheet>
			<SheetTrigger render={<Button variant="outline">Show details</Button>} />
			<SheetContent
				side="right"
				showCloseButton={false}
				className="min-w-1/2 p-4"
			>
				<SheetHeader>
					<SheetTitle className="font-medium text-base text-foreground">
						Sale details
					</SheetTitle>
				</SheetHeader>
				<Card className="m-auto w-full max-w-sm">
					<CardHeader>
						<CardTitle>{listing?.instrument_name}</CardTitle>
						<CardDescription className="flex gap-2">
							<Badge className="rounded-sm font-mono text-sm tracking-wider">
								{listing?.ticker}
							</Badge>
							{listing?.exchange_mic && MIC_LABEL[listing?.exchange_mic]}
						</CardDescription>
					</CardHeader>
					<CardContent className="grid gap-2">
						<div className="flex items-baseline justify-between">
							<p>Quantity</p>
							<p className="font-semibold text-base">{10}</p>
						</div>
						<div className="flex items-baseline justify-between">
							<p>Unit Price</p>
							<p className="font-semibold text-base">{formatCurrency(30)}</p>
						</div>

						<Separator />

						<div className="flex items-baseline justify-between">
							<p>Total</p>
							<p className="font-semibold text-base">{formatCurrency(300)}</p>
						</div>
					</CardContent>
				</Card>
				<div>
					<p className="font-medium text-base">Per lot</p>
					<div className="grid gap-2">
						{computation.allocations.map((a) => {
							return (
								<div key={a.origin_lot_id}>
									<p>Acquired on</p>
									<p>
										{dateformatters.fulldate.format(
											new Date(a.acquisition_date),
										)}
									</p>

									<p>Quantity</p>
									<p>{a.quantity}</p>

									{a.tax?.pre2026_cost_basis && (
										<>
											<p>Method</p>
											<p>{METHOD_LABEL[a.tax.pre2026_cost_basis.method]}</p>

											<p>Buy price (for CGT)</p>
											<p>{formatCurrency(Number(a.tax.buy_price_eur))}</p>
										</>
									)}

									<p>Taxable gains</p>
									<p>{formatCurrency(Number(a.tax?.taxable_gain_eur))}</p>
								</div>
							);
						})}
					</div>
					{/* <Table> */}
					{/* 	<TableHeader> */}
					{/* 		<TableRow> */}
					{/* 			<TableHead>Acquired</TableHead> */}
					{/* 			<TableHead>Qty</TableHead> */}
					{/* 			{computation.subject_to_cgt && ( */}
					{/* 				<TableHead>Taxable gain</TableHead> */}
					{/* 			)} */}
					{/* 			{computation.subject_to_cgt && <TableHead>Method</TableHead>} */}
					{/* 		</TableRow> */}
					{/* 	</TableHeader> */}
					{/* 	<TableBody> */}
					{/* 		{computation.allocations.map((a) => ( */}
					{/* 			<TableRow key={a.origin_lot_id}> */}
					{/* 				<TableCell>{a.acquisition_date}</TableCell> */}
					{/* 				<TableCell>{a.quantity}</TableCell> */}
					{/* 				{computation.subject_to_cgt && ( */}
					{/* 					<TableCell> */}
					{/* 						{a.tax */}
					{/* 							? formatCurrency(Number(a.tax.taxable_gain_eur)) */}
					{/* 							: "—"} */}
					{/* 					</TableCell> */}
					{/* 				)} */}
					{/* 				{computation.subject_to_cgt && ( */}
					{/* 					<TableCell> */}
					{/* 						{a.tax?.pre2026_cost_basis */}
					{/* 							? METHOD_LABEL[a.tax.pre2026_cost_basis.method] */}
					{/* 							: "Post-2025 (FIFO lot price)"} */}
					{/* 					</TableCell> */}
					{/* 				)} */}
					{/* 			</TableRow> */}
					{/* 		))} */}
					{/* 	</TableBody> */}
					{/* </Table> */}
					<p className="mt-8 font-medium text-xl">Totals</p>
					<div className="grid gap-2">
						<div>
							<p className="py-1 font-medium">
								Economic gain (incl. taxes & fees)
							</p>
							<p>{computation.total_economic_gain_eur}</p>
						</div>
						<div>
							<p className="font-medium">Taxable gain (excl. taxes & fees)</p>
							<p>{computation.total_taxable_gain_eur}</p>
						</div>
					</div>
				</div>
			</SheetContent>
		</Sheet>
	);
}
