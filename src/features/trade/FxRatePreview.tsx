import { cn, formatCurrency } from "@/lib/utils";

export function FxRatePreview({
	isLoading = false,
	error = null,
	rate,
	convertedValue,
	className,
}: {
	isLoading?: boolean;
	error?: Error | null;
	rate: string | undefined;
	convertedValue: number | null;
	className?: string;
}) {
	if (isLoading) {
		return (
			<p className={cn("pl-4 text-muted-foreground text-xs", className)}>
				Loading exchange rate…
			</p>
		);
	}
	if (error) {
		return (
			<p className={cn("pl-4 text-destructive text-xs", className)}>
				{error.message}
			</p>
		);
	}
	if (!rate || convertedValue === null) return null;

	return (
		<div
			className={cn("flex items-center justify-between gap-3 pl-4", className)}
		>
			<span className="text-muted-foreground text-sm">Converted to EUR</span>
			<span className="font-medium text-sm">
				{formatCurrency(convertedValue)}
			</span>
		</div>
	);
}
