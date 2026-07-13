import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/listings/add")({
	component: RouteComponent,
	staticData: { title: "Add" },
});

function RouteComponent() {
	return <div>Hello "/listing/add"!</div>;
}
