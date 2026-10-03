import { Camera, ListStart, Rewind, SkipBack } from "lucide-react";
import type { ReactNode } from "react";
import type {
	AllocationTax,
	ListingInfo,
	Pre2026CostBasisMethod,
	SellComputation,
} from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
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
	TableFooter,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { useDateFormatters } from "@/hooks/use-date-formatters";
import { formatCurrency, MIC_LABEL } from "@/lib/utils";

const METHOD_LABEL: Record<
	Pre2026CostBasisMethod,
	{ icon: ReactNode; label: string }
> = {
	FOTOMOMENT: { icon: <Camera />, label: "Fotomoment (31.12.2025)" },
	HISTORICAL_ELECTED: { icon: <Rewind />, label: "Historical cost (elected)" },
	FLOORED_AT_ZERO: {
		icon: <SkipBack />,
		label: "Historical cost, floored to zero",
	},
};

function methodLabel(tax: AllocationTax): { icon: ReactNode; label: string } {
	if (!tax.pre2026_cost_basis) {
		return { icon: <ListStart />, label: "FIFO (post-2025)" };
	}
	return METHOD_LABEL[tax.pre2026_cost_basis.method];
}

export function SellTaxDetailSheet({
	computation,
	listings,
	sale,
}: {
	computation: SellComputation;
	listings: ListingInfo[];
	sale: any;
}) {
	const dateformatters = useDateFormatters();
	// TODO: be specific about listing (id instead of isin)
	const listing = listings.find((l) => l.isin === computation.isin);
	const estimatedCGT = Number(computation.estimated_gross_cgt_eur ?? 0);

	return (
		<Sheet>
			<SheetTrigger render={<Button variant="outline">Show details</Button>} />
			<SheetContent side="right" showCloseButton={false} className="min-w-fit">
				<SheetHeader>
					<SheetTitle className="font-medium text-base text-foreground">
						Sale details
					</SheetTitle>
				</SheetHeader>
				<div className="grid gap-6 px-4">
					<Card className="max-w-md">
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
								<p className="font-semibold text-base">{sale.quantity}</p>
							</div>
							<div className="flex items-baseline justify-between">
								<p>Unit Price</p>
								<p className="font-semibold text-base">
									{formatCurrency(
										Number(sale.unitPrice.amount),
										sale.unitPrice.currency,
									)}
								</p>
							</div>

							<Separator />

							<div className="flex items-baseline justify-between">
								<p>Total</p>
								<p className="font-semibold text-base">
									{formatCurrency(sale.base, sale.unitPrice.currency)}
								</p>
							</div>
						</CardContent>
					</Card>

					<div className="grid gap-2">
						<p className="font-medium text-base">FIFO Lot Breakdown</p>
						<div className="max-w-md overflow-hidden rounded-md border border-foreground/10">
							<Table>
								<TableHeader>
									<TableRow>
										<TableHead>Acquired</TableHead>
										<TableHead>Qty</TableHead>
										<TableHead>Cost basis</TableHead>
										{computation.subject_to_cgt && (
											<TableHead>Taxable gain</TableHead>
										)}
										{computation.subject_to_cgt && (
											<TableHead>Method</TableHead>
										)}
									</TableRow>
								</TableHeader>
								<TableBody>
									{computation.allocations.map((a) => (
										<TableRow key={a.origin_lot_id}>
											<TableCell>
												{dateformatters.fulldate.format(
													new Date(a.acquisition_date),
												)}
											</TableCell>
											<TableCell>{a.quantity}</TableCell>
											<TableCell>
												{formatCurrency(Number(a.tax?.taxable_buy_price_eur))}
											</TableCell>
											{computation.subject_to_cgt && (
												<TableCell className="text-emerald-700">
													{a.tax
														? formatCurrency(Number(a.tax.taxable_gain_eur))
														: "—"}
												</TableCell>
											)}
											{computation.subject_to_cgt && (
												<TableCell>
													{a.tax
														? (() => {
																const { icon, label } = methodLabel(a.tax);
																return (
																	<span className="flex items-center gap-1.5">
																		{icon}
																		{label}
																	</span>
																);
															})()
														: "—"}
												</TableCell>
											)}
										</TableRow>
									))}
								</TableBody>
								<TableFooter>
									<TableRow>
										<TableCell>Total</TableCell>
										<TableCell>
											<p>
												{computation.allocations.reduce(
													(acc, c) => acc + Number(c.quantity),
													0,
												)}
											</p>
										</TableCell>
										<TableCell>
											{formatCurrency(
												Number(
													computation.allocations.reduce(
														(acc, c) =>
															acc + Number(c.tax?.taxable_buy_price_eur),
														0,
													),
												),
											)}
										</TableCell>
										<TableCell className="text-emerald-700">
											{formatCurrency(
												Number(computation.total_taxable_gain_eur),
											)}
										</TableCell>
										<TableCell></TableCell>
									</TableRow>
								</TableFooter>
							</Table>
						</div>
					</div>

					<div className="grid gap-2">
						<p className="font-medium text-base">Summary</p>
						<Card className="max-w-md">
							<CardContent className="grid gap-2">
								<div className="flex items-baseline justify-between">
									<p>Gross</p>
									{/* TODO: fx */}
									<p className="font-semibold text-base text-emerald-700">
										{formatCurrency(sale.base, sale.unitPrice.currency)}
									</p>
								</div>
								<div className="space-y-0.5">
									<div className="flex items-baseline justify-between">
										<p>Taxes & Fees</p>
										{/* TODO: fx */}
										<p className="font-semibold text-base text-rose-700">
											{formatCurrency(
												Number(sale.brokerFee.amount) +
													Number(sale.tobFee.amount) +
													estimatedCGT,
											)}
										</p>
									</div>
									<div className="flex items-baseline justify-between gap-3 pl-4">
										<span className="text-muted-foreground text-sm">
											Broker
										</span>
										{/* TODO: fx */}
										<span className="font-medium text-rose-700 text-sm">
											{formatCurrency(
												Number(sale.brokerFee.amount),
												sale.brokerFee.currency,
											)}
										</span>
									</div>

									<div className="flex items-baseline justify-between gap-3 pl-4">
										<span className="text-muted-foreground text-sm">TOB</span>
										<span className="font-medium text-rose-700 text-sm">
											{formatCurrency(Number(sale.tobFee.amount))}
										</span>
									</div>

									<div className="flex items-baseline justify-between gap-3 pl-4">
										<div className="grid">
											<span className="text-muted-foreground text-sm">
												CGT (estimated)
											</span>
											<span className="text-muted-foreground text-xs">
												Before annual exemption and loss netting.
											</span>
										</div>
										<span className="font-medium text-rose-700 text-sm">
											{formatCurrency(estimatedCGT)}
										</span>
									</div>
								</div>

								<Separator />

								<div className="flex items-baseline justify-between">
									<p>Net</p>
									{/* TODO: fx */}
									<p className="font-semibold text-base text-emerald-700">
										{formatCurrency(
											sale.base -
												Number(sale.brokerFee.amount) -
												Number(sale.tobFee.amount) -
												estimatedCGT,
										)}
									</p>
								</div>
							</CardContent>
						</Card>
					</div>
				</div>
			</SheetContent>
		</Sheet>
	);
}
