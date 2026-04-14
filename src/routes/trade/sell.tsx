import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/trade/sell")({
	component: RouteComponent,
	staticData: { title: "Sell" },
});

function RouteComponent() {
	return <div>Hello "/trade/sell"!</div>;
}
