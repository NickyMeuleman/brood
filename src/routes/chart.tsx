import { createFileRoute } from "@tanstack/react-router";
import Page from "@/features/chart/page.tsx";

export const Route = createFileRoute("/chart")({
	component: About,
});

function About() {
	return <Page />;
}
