import { Camera, ListStart, Rewind, SkipBack } from "lucide-react";
import type {
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

const METHOD_LABEL: Record<Pre2026CostBasisMethod, any> = {
	FOTOMOMENT: <Camera />,
	HISTORICAL_ELECTED: <Rewind />,
	FLOORED_AT_ZERO: <SkipBack />,
};

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
									{formatCurrency(Number(sale.unitPrice))}
								</p>
							</div>

							<Separator />

							<div className="flex items-baseline justify-between">
								<p>Total</p>
								<p className="font-semibold text-base">
									{formatCurrency(sale.base)}
								</p>
							</div>
						</CardContent>
					</Card>

					<div className="grid gap-2">
						<p className="font-medium text-base">FIFO Lot Breakdown</p>
						<div className="overflow-hidden rounded-md border border-foreground/10">
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
												{formatCurrency(Number(a.tax?.buy_price_eur))}
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
													{a.tax?.pre2026_cost_basis ? (
														METHOD_LABEL[a.tax.pre2026_cost_basis.method]
													) : (
														<ListStart />
													)}
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
														(acc, c) => acc + Number(c.tax?.buy_price_eur),
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
									<p className="font-semibold text-base text-emerald-700">
										{formatCurrency(sale.base)}
									</p>
								</div>
								<div className="space-y-0.5">
									<div className="flex items-baseline justify-between">
										<p>Taxes & Fees</p>
										<p className="font-semibold text-base text-rose-700">
											{formatCurrency(
												Number(sale.brokerFee) +
													Number(sale.tobFee) +
													Number(computation.total_taxable_gain_eur ?? 0) * 0.1,
											)}
										</p>
									</div>
									<div className="flex items-baseline justify-between gap-3 pl-4">
										<span className="text-muted-foreground text-sm">
											Broker
										</span>
										<span className="font-medium text-rose-700 text-sm">
											{formatCurrency(Number(sale.brokerFee))}
										</span>
									</div>

									<div className="flex items-baseline justify-between gap-3 pl-4">
										<span className="text-muted-foreground text-sm">TOB</span>
										<span className="font-medium text-rose-700 text-sm">
											{formatCurrency(Number(sale.tobFee))}
										</span>
									</div>

									<div className="flex items-baseline justify-between gap-3 pl-4">
										<span className="text-muted-foreground text-sm">CGT</span>
										<span className="font-medium text-rose-700 text-sm">
											{formatCurrency(
												Number(computation.total_taxable_gain_eur ?? 0) * 0.1,
											)}
										</span>
									</div>
								</div>

								<Separator />

								<div className="flex items-baseline justify-between">
									<p>Net</p>
									<p className="font-semibold text-base text-emerald-700">
										{formatCurrency(
											sale.base -
												Number(sale.brokerFee) -
												Number(sale.tobFee) -
												Number(computation.total_taxable_gain_eur ?? 0) * 0.1,
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
