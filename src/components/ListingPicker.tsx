import type { ListingInfo } from "@/bindings";
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
import { useFieldContext } from "@/hooks/form-context";
import { cn, getCurrencySymbol, MIC_LABEL } from "@/lib/utils";
import { Badge } from "./ui/badge";

export const ListingPicker = ({
	label,
	listings,
	isLoading,
}: {
	label: string;
	listings: ListingInfo[];
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
				onValueChange={(id) => field.handleChange(id ?? 1)}
				itemToStringLabel={(id) =>
					listings.find((l) => l.id === id)?.ticker ?? ""
				}
			>
				<SelectTrigger disabled={isLoading}>
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						{listings.map((l) => {
							const exchangeLabel = MIC_LABEL[l.exchange_mic] ?? l.exchange_mic;
							const currencySymbol = getCurrencySymbol(l.currency_code);
							const isForeignCurrency = l.currency_code !== "EUR";
							return (
								<SelectItem key={l.id} value={l.id}>
									<div className="flex w-full items-center gap-3">
										<div className="flex min-w-0 flex-1 flex-col gap-0.5">
											<TruncatedTooltip>{l.instrument_name}</TruncatedTooltip>
											<div className="flex items-center gap-1.5 whitespace-nowrap font-normal text-muted-foreground text-sm">
												<Badge
													variant="ghost"
													className={cn(
														"bg-secondary font-mono text-secondary-foreground text-sm tracking-wider",
													)}
												>
													{l.ticker}
												</Badge>
												<span className="opacity-60">·</span>
												<span>{exchangeLabel}</span>
												{isForeignCurrency && (
													<>
														<span className="opacity-60">·</span>
														<span>{currencySymbol}</span>
													</>
												)}
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
