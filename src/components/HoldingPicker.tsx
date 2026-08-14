import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { TruncatedTooltip } from "@/features/holdings/TruncatedTooltip";
import type { SellableHolding } from "@/features/trade/types";
import { useFieldContext } from "@/hooks/form-context";
import { cn, getCurrencySymbol, MIC_LABEL } from "@/lib/utils";
import { Badge } from "./ui/badge";

export const HoldingPicker = ({
	label,
	holdings,
	isLoading,
}: {
	label: string;
	holdings: SellableHolding[];
	isLoading: boolean;
}) => {
	const field = useFieldContext<number>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<Select
				id={field.name}
				value={field.state.value}
				onValueChange={(id) => field.handleChange(id ?? 0)}
				onOpenChange={(open) => {
					if (!open) field.handleBlur();
				}}
				itemToStringLabel={(id) =>
					holdings.find((h) => h.listing_id === id)?.ticker ?? ""
				}
			>
				<SelectTrigger disabled={isLoading}>
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						{holdings.map((h) => {
							const exchangeLabel = MIC_LABEL[h.exchange_mic] ?? h.exchange_mic;
							const currencySymbol = getCurrencySymbol(h.currency_code);
							const isForeignCurrency = h.currency_code !== "EUR";

							return (
								<SelectItem key={h.listing_id} value={h.listing_id}>
									<div className="flex w-full items-center gap-3">
										<div className="flex min-w-0 flex-1 flex-col gap-0.5">
											<TruncatedTooltip>{h.instrument_name}</TruncatedTooltip>
											<div className="flex items-center gap-1.5 whitespace-nowrap font-normal text-muted-foreground text-sm">
												<Badge
													variant="ghost"
													className={cn(
														"bg-secondary font-mono text-secondary-foreground text-sm tracking-wider",
													)}
												>
													{h.ticker}
												</Badge>
												<span className="opacity-60">·</span>
												<span>{exchangeLabel}</span>
												{isForeignCurrency && (
													<>
														<span className="opacity-60">·</span>
														<span>{currencySymbol}</span>
													</>
												)}
												<span className="opacity-60">·</span>
												<span>{h.quantity} held</span>
											</div>
										</div>
									</div>
								</SelectItem>
							);
						})}
					</SelectGroup>
				</SelectContent>
			</Select>
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
};
