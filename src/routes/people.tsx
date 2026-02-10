import { createFileRoute } from "@tanstack/react-router";
import PeoplePage from "@/features/people/page.tsx";

export const Route = createFileRoute("/people")({
	component: RouteComponent,
});

function RouteComponent() {
	return <PeoplePage />;
}
