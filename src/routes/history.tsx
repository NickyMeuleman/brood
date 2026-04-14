import { createFileRoute } from "@tanstack/react-router";
import Page from "@/features/chart/page.tsx";

export const Route = createFileRoute("/history")({
	component: RouteComponent,
	staticData: { title: "History" },
});

function RouteComponent() {
	return <Page />;
}
