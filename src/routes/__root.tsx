import { TanStackDevtools } from "@tanstack/react-devtools";
import { FormDevtoolsPanel } from "@tanstack/react-form-devtools";
import { ReactQueryDevtoolsPanel } from "@tanstack/react-query-devtools";
import { createRootRoute, Outlet, useMatches } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { AppShell } from "@/features/appshell/page";
// import { ReactTableDevtoolsPanel } from "@tanstack/react-table-devtools";

export default function RouteComponent() {
	const matches = useMatches();

	return (
		<>
			<AppShell matches={matches.filter((m) => m.staticData?.title)}>
				<Outlet />
			</AppShell>
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
