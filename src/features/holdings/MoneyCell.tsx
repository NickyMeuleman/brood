import { ArrowLeftRight } from "lucide-react";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn, formatCurrency } from "@/lib/utils";

export function MoneyCell({
	value,
	currency,
	isConverted,
	originalValue,
	originalCurrency,
	className,
	numFormatOpts,
}: {
	value: number;
	currency: string;
	isConverted: boolean;
	originalValue?: number;
	originalCurrency?: string;
	className?: string;
	numFormatOpts?: Intl.NumberFormatOptions;
}) {
	if (!isConverted) {
		return (
			<div className={cn("text-right font-medium", className)}>
				{formatCurrency(value, currency, numFormatOpts)}
			</div>
		);
	}

	return (
		<Tooltip>
			<TooltipTrigger
				render={
					<div
						className={cn(
							"flex cursor-default items-center justify-end gap-1 font-medium",
							className,
						)}
					>
						<ArrowLeftRight className="inline h-3 w-3 shrink-0 text-muted-foreground/50" />
						<span className="underline decoration-muted-foreground/40 decoration-dashed underline-offset-2">
							{formatCurrency(value, currency, numFormatOpts)}
						</span>
					</div>
				}
			/>
			<TooltipContent side="top" className="text-xs">
				<p className="font-medium">
					{formatCurrency(
						originalValue ?? 0,
						originalCurrency ?? "EUR",
						numFormatOpts,
					)}
					<span className="text-muted-foreground"> in {originalCurrency}</span>
				</p>
			</TooltipContent>
		</Tooltip>
	);
}
