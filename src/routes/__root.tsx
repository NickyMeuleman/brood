import { TanStackDevtools } from "@tanstack/react-devtools";
import { FormDevtoolsPanel } from "@tanstack/react-form-devtools";
import { ReactQueryDevtoolsPanel } from "@tanstack/react-query-devtools";
import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";

// import { ReactTableDevtoolsPanel } from "@tanstack/react-table-devtools";
import { AppSidebar } from "@/components/app-sidebar";
import {
	Breadcrumb,
	BreadcrumbItem,
	BreadcrumbLink,
	BreadcrumbList,
	BreadcrumbPage,
	BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Separator } from "@/components/ui/separator";
import {
	SidebarInset,
	SidebarProvider,
	SidebarTrigger,
} from "@/components/ui/sidebar";

export default function Page() {
	return (
		<SidebarProvider>
			<AppSidebar />
			<SidebarInset>
				<header className="flex h-16 shrink-0 items-center gap-2 border-b">
					<div className="flex items-center gap-2 px-3">
						<SidebarTrigger />
						<Separator orientation="vertical" className="mr-2 h-4" />
						<Breadcrumb>
							<BreadcrumbList>
								<BreadcrumbItem className="hidden md:block">
									<BreadcrumbLink href="#">
										Build Your Application
									</BreadcrumbLink>
								</BreadcrumbItem>
								<BreadcrumbSeparator className="hidden md:block" />
								<BreadcrumbItem>
									<BreadcrumbPage>Data Fetching</BreadcrumbPage>
								</BreadcrumbItem>
							</BreadcrumbList>
						</Breadcrumb>
					</div>
				</header>
				<Outlet />
			</SidebarInset>

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
		</SidebarProvider>
	);
}

// const RootLayout = () => (
// 	<>
// 		<div className="flex gap-2 p-2">
// 			<Link to="/" className="data-[status=active]:font-bold">
// 				Home
// 			</Link>
// 			<Link to="/people" className="data-[status=active]:font-bold">
// 				People
// 			</Link>
// 			<Link to="/chart" className="data-[status=active]:font-bold">
// 				Chart
// 			</Link>
// 			<Link to="/admin" className="data-[status=active]:font-bold">
// 				Admin
// 			</Link>
// 		</div>
// 		<hr />
// 		<Outlet />
// 	</>
// );

export const Route = createRootRoute({
	component: Page,
});
