import type { Period } from "@/bindings";
import { PERIOD_LABEL } from "@/lib/utils";

export function Label({
	label,
	period,
	showPeriod,
}: {
	label?: string;
	period?: Period;
	showPeriod?: boolean;
}) {
	return (
		<p className="flex items-baseline gap-0.5">
			<span>{label}</span>
			{showPeriod && period ? (
				<span className="text-muted-foreground text-xs uppercase tracking-widest">
					({PERIOD_LABEL[period]})
				</span>
			) : null}
		</p>
	);
}
