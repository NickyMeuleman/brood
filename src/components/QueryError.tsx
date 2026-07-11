import { AlertCircle } from "lucide-react";

export function QueryError({ message }: { message: string }) {
	return (
		<div className="flex flex-col items-center gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-6 text-center text-destructive">
			<AlertCircle className="h-5 w-5 shrink-0" />
			<p className="font-medium text-sm">{message}</p>
		</div>
	);
}
