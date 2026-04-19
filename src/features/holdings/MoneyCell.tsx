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
	value: number | null;
	currency: string;
	isConverted: boolean;
	originalValue: number | null;
	originalCurrency?: string;
	className?: string;
	numFormatOpts?: Intl.NumberFormatOptions;
}) {
	if (value == null) {
		return (
			<div
				className={cn(
					"text-right font-medium text-muted-foreground",
					className,
				)}
			>
				Missing data
			</div>
		);
	}

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
				{originalValue !== null ? (
					<p className="font-medium">
						{formatCurrency(
							originalValue,
							originalCurrency ?? "EUR",
							numFormatOpts,
						)}
						<span className="opacity-60">
							{" "}
							in {originalCurrency}
						</span>
					</p>
				) : (
					<p
						className={cn(
							"text-right font-medium text-muted-foreground",
							className,
						)}
					>
						Missing data
					</p>
				)}
			</TooltipContent>
		</Tooltip>
	);
}
