import type { Pre2026CostBasisMethod, SellComputation } from "@/bindings";
import { Button } from "@/components/ui/button";
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
import { formatCurrency } from "@/lib/utils";

const METHOD_LABEL: Record<Pre2026CostBasisMethod, string> = {
	FOTOMOMENT: "Fotomoment (2025-12-31)",
	HISTORICAL_ELECTED: "Historical cost",
	FLOORED_AT_ZERO: "Historical cost, floored to zero",
};

export function SellTaxDetailSheet({
	computation,
}: {
	computation: SellComputation;
}) {
	const dateformatters = useDateFormatters();

	return (
		<Sheet>
			<SheetTrigger
				render={<Button variant="outline">Show tax details</Button>}
			/>
			<SheetContent
				side="right"
				showCloseButton={false}
				className="min-w-fit p-4"
			>
				<SheetHeader>
					<SheetTitle className="font-medium text-base text-foreground">
						FIFO / tax breakdown
					</SheetTitle>
				</SheetHeader>
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
