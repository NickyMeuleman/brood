import { createFileRoute } from "@tanstack/react-router";
import { commands } from "@/bindings";
import { Button } from "@/components/ui/button";

export const Route = createFileRoute("/admin")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div>
			<Button
				onClick={() => {
					commands.syncPrices();
				}}
			>
				Sync all prices
			</Button>
			<Button
				onClick={() => {
					commands.syncFx();
				}}
			>
				Sync all fx
			</Button>
		</div>
	);
}
