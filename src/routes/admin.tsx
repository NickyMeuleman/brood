import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/admin")({
	component: Admin,
});

function Admin() {
	return <div className="p-5">do admin things</div>;
}
