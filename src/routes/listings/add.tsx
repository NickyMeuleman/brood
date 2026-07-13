import { createFileRoute } from "@tanstack/react-router";
import Page from "@/features/listings/add.tsx";

export const Route = createFileRoute("/listings/add")({
	component: RouteComponent,
	staticData: { title: "Add" },
});

function RouteComponent() {
	return <Page />;
}
