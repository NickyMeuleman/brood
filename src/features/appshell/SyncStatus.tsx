import type { UseQueryResult } from "@tanstack/react-query";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";

export function SyncStatus({
	sync,
}: {
	sync: UseQueryResult<SyncOutcomes, Error>;
}) {
	const { isLoading, isError, data, dataUpdatedAt } = sync;
	if (!isLoading && !isError && !data) return null;

	return (
		<TooltipProvider>
			<Tooltip>
				<TooltipTrigger
					render={
						<div className="flex cursor-default select-none items-center gap-2 rounded-md px-2 py-1">
							{isLoading && (
								<RefreshCw className="h-4 w-4 animate-spin text-muted-foreground" />
							)}

							{isError && <AlertCircle className="h-4 w-4 text-destructive" />}

							{!isLoading && !isError && data && (
								<CheckCircle2 className="h-4 w-4 text-emerald-500 transition-opacity duration-1000" />
							)}
						</div>
					}
				/>
				<TooltipContent side="bottom" align="end">
					<div>
						<p>
							{isLoading && "Syncing market data..."}
							{isError && "Sync failed. Check your connection."}
							{!isLoading && !isError && "Market data up to date"}
						</p>
						<p>Last synced on: {new Date(dataUpdatedAt).toDateString()}</p>
					</div>
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	);
}
