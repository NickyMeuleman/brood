import { createFileRoute } from "@tanstack/react-router";
import Page from "@/features/people/page.tsx";

export const Route = createFileRoute("/trade/")({
	component: RouteComponent,
});

function RouteComponent() {
	return <Page />;
}
