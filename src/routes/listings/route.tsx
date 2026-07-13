import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/listings")({
	component: RouteComponent,
	staticData: { title: "Listing" },
});

function RouteComponent() {
	return <Outlet />;
}
