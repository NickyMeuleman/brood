import { createFileRoute } from "@tanstack/react-router";
import Page from "@/features/trade/BuyPage";

export const Route = createFileRoute("/trade/buy")({
	component: RouteComponent,
	staticData: { title: "Buy" },
});

function RouteComponent() {
	return <Page />
}
