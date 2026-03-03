import { createFileRoute } from "@tanstack/react-router";
import Holdings from "@/features/holdings/page.tsx";

export const Route = createFileRoute("/")({
	component: Index,
});

function Index() {
	return (
		<div className="p-2">
			<Holdings />
		</div>
	);
}
