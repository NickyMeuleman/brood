import { type AnyRouteMatch, Link } from "@tanstack/react-router";
import { Fragment } from "react";
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
import { SyncStatus } from "./SyncStatus";

export function AppShell({
	children,
	matches,
}: {
	children: React.ReactNode;
	matches: AnyRouteMatch[];
}) {
	return (
		<SidebarProvider>
			<AppSidebar />
			<SidebarInset>
				<header className="flex h-16 shrink-0 items-center justify-between gap-2 border-b px-3">
					<div className="flex items-center gap-2 px-3">
						<SidebarTrigger />
						<Separator
							orientation="vertical"
							className="h-4 data-vertical:self-center"
						/>
						<Breadcrumb>
							<BreadcrumbList>
								{matches.map((match, index) => {
									const isLast = index === matches.length - 1;
									const title = match.staticData?.title ?? match.id;

									return (
										<Fragment key={match.id}>
											<BreadcrumbSeparator className="hidden first:hidden md:block" />
											<BreadcrumbItem className="hidden last:block md:block">
												{isLast ? (
													<BreadcrumbPage>{title}</BreadcrumbPage>
												) : (
													<BreadcrumbLink
														render={
															<Link to={match.fullPath} params={match.params}>
																{title}
															</Link>
														}
													/>
												)}
											</BreadcrumbItem>
										</Fragment>
									);
								})}
							</BreadcrumbList>
						</Breadcrumb>
					</div>
					<SyncStatus />
				</header>
				<div className="m-4">{children}</div>
			</SidebarInset>
		</SidebarProvider>
	);
}
