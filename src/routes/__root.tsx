import { TanStackDevtools } from "@tanstack/react-devtools";
import { FormDevtoolsPanel } from "@tanstack/react-form-devtools";
import { ReactQueryDevtoolsPanel } from "@tanstack/react-query-devtools";
import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";

// import { ReactTableDevtoolsPanel } from "@tanstack/react-table-devtools";

const RootLayout = () => (
	<>
		<div className="flex gap-2 p-2">
			<Link to="/" className="data-[status=active]:font-bold">
				Home
			</Link>
			<Link to="/people" className="data-[status=active]:font-bold">
				People
			</Link>
			<Link to="/chart" className="data-[status=active]:font-bold">
				Chart
			</Link>
			<Link to="/admin" className="data-[status=active]:font-bold">
				Admin
			</Link>
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
