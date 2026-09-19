import { TanStackDevtools } from "@tanstack/react-devtools";
import { FormDevtoolsPanel } from "@tanstack/react-form-devtools";
import { ReactQueryDevtoolsPanel } from "@tanstack/react-query-devtools";
import { createRootRoute, Outlet, useMatches } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { useEffect } from "react";
import { Toaster } from "@/components/ui/sonner";
import { AppShell } from "@/features/appshell/page";
import { useSync } from "@/hooks/use-sync";
// import { ReactTableDevtoolsPanel } from "@tanstack/react-table-devtools";

export default function RouteComponent() {
	const matches = useMatches();
	const sync = useSync();

	// biome-ignore lint/correctness/useExhaustiveDependencies: run once at startup
	useEffect(() => {
    // off during dev to avoid hammering API
		// sync.mutate();
	}, []);

	return (
		<>
			<AppShell matches={matches.filter((m) => m.staticData?.title)}>
				<Outlet />
			</AppShell>
			<Toaster position="top-center" richColors />
			<TanStackDevtools
				plugins={[
					{
						name: "TanStack Query",
						render: <ReactQueryDevtoolsPanel />,
						defaultOpen: true,
					},
					{
						name: "TanStack Router",
						render: <TanStackRouterDevtoolsPanel />,
						defaultOpen: false,
					},
					{
						name: "TanStack Form",
						render: <FormDevtoolsPanel />,
						defaultOpen: false,
					},
					// {
					// 	name: "TanStack Table",
					// 	// has to have access to the table variable, use locally or store the entire table in a context?
					// 	render: <ReactTableDevtoolsPanel />,
					// 	defaultOpen: false,
					// },
				]}
			/>
		</>
	);
}

export const Route = createRootRoute({
	component: RouteComponent,
	staticData: { title: "Brood" },
});
