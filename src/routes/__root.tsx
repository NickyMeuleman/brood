import { TanStackDevtools } from "@tanstack/react-devtools";
import { FormDevtoolsPanel } from "@tanstack/react-form-devtools";
import { ReactQueryDevtoolsPanel } from "@tanstack/react-query-devtools";
import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";

// import { ReactTableDevtoolsPanel } from "@tanstack/react-table-devtools";

const RootLayout = () => (
	<>
		<div className="flex gap-2 p-2">
			<Link to="/" className="data-[status=active]:bg-red-700">
				Home
			</Link>
			<Link to="/about">About</Link>
			<Link to="/people">People</Link>
			<Link to="/payments">Payments</Link>
			<Link to="/chart">Chart</Link>
		</div>
		<hr />
		<Outlet />
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

export const Route = createRootRoute({
	component: RootLayout,
});
