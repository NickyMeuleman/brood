import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/history")({
	component: RouteComponent,
	staticData: { title: "History" },
});

function RouteComponent() {
	return <div>history</div>;
}
