import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";

const RootLayout = () => (
	<>
		<div className="p-2 flex gap-2">
			<Link to="/" activeProps={{ className: "text-bold text-xl" }}>
				Home
			</Link>
			<Link to="/about" className="text-lg">
				About
			</Link>
		</div>
		<hr />
		<Outlet />
		<TanStackRouterDevtools />
	</>
);

export const Route = createRootRoute({
	component: RootLayout,
});
