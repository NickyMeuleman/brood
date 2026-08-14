import { createFileRoute } from "@tanstack/react-router";
import Page from "@/features/trade/SellPage";

export const Route = createFileRoute("/trade/sell")({
	component: RouteComponent,
	staticData: { title: "Sell" },
});

function RouteComponent() {
	return <Page />;
}
