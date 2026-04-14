import { createFileRoute } from "@tanstack/react-router";
import Holdings from "@/features/holdings/page.tsx";

export const Route = createFileRoute("/")({
	component: RouteComponent,
	staticData: { title: "Holdings" },
});

function RouteComponent() {
	return <Holdings />;
}
