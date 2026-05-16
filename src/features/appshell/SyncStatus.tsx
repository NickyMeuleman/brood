import { AlertCircle, CheckCircle2, RefreshCw } from "lucide-react";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { useSyncStore } from "@/stores/sync";

export function SyncStatus() {
	const { status, submittedAt, error } = useSyncStore();
	if (status === "idle") return null;

	const isLoading = status === "pending";
	const isError = status === "error";
	const isSuccess = status === "success";

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

							{isSuccess && (
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
						{submittedAt && (
							<p>Last synced at: {new Date(submittedAt).toDateString()}</p>
						)}
						{isError && error && (
							<p className="text-destructive">{error.message}</p>
						)}
					</div>
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	);
}
