import { TanStackDevtools } from "@tanstack/react-devtools";
import { FormDevtoolsPanel } from "@tanstack/react-form-devtools";
import { ReactQueryDevtoolsPanel } from "@tanstack/react-query-devtools";
import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";

const RootLayout = () => (
	<>
		<div className="p-2 flex gap-2">
			<Link to="/" className="data-[status=active]:bg-red-700">
				Home
			</Link>
			<Link to="/about">About</Link>
			<Link to="/people">People</Link>
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
			]}
		/>
	</>
);

export const Route = createRootRoute({
	component: RootLayout,
});
