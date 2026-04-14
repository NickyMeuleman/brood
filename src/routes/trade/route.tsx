import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/trade")({
	component: RouteComponent,
	staticData: { title: "Trade" },
});

function RouteComponent() {
	return <Outlet />;
}
