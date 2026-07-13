import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/trade/")({
	component: RouteComponent,
});

function RouteComponent() {
	return <div>trade index</div>;
}
