import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/trade/buy")({
	component: RouteComponent,
	staticData: { title: "Buy" },
});

function RouteComponent() {
	return <div>Hello "/trade/buy"!</div>;
}
