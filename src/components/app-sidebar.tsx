import { Link, type LinkOptions, useMatchRoute } from "@tanstack/react-router";
import { Euro } from "lucide-react";
import type * as React from "react";
import {
	Sidebar,
	SidebarContent,
	SidebarGroup,
	SidebarHeader,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
	SidebarMenuSub,
	SidebarMenuSubButton,
	SidebarMenuSubItem,
	SidebarRail,
} from "@/components/ui/sidebar";

type NavItem = { title: string; items?: NavItem[] } & LinkOptions;
const navData: NavItem[] = [
	{
		title: "Holdings",
		to: "/",
	},
	{
		title: "History",
		to: "/history",
	},
	{
		title: "Trade",
		to: "/trade",
		items: [
			{ title: "Buy", to: "/trade/buy" },
			{ title: "Sell", to: "/trade/sell" },
		],
	},
	{
		title: "Listing",
		to: "/listings",
		items: [{ title: "Add", to: "/listings/add" }],
	},
	{ title: "Admin", to: "/admin" },
];

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
	const matchRoute = useMatchRoute();

	return (
		<Sidebar {...props}>
			<SidebarHeader>
				<SidebarMenu>
					<SidebarMenuItem>
						<SidebarMenuButton size="lg" render={<Link to="." />}>
							<div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
								<Euro />
							</div>
							<div className="flex flex-col gap-0.5 leading-none">
								<span className="font-medium">Brood</span>
								<span className="text-xs">Met beleg</span>
							</div>
						</SidebarMenuButton>
					</SidebarMenuItem>
				</SidebarMenu>
			</SidebarHeader>
			<SidebarContent>
				<SidebarGroup>
					<SidebarMenu>
						{navData.map((item) => {
							const active = !!matchRoute({
								to: item.to,
								fuzzy: item.to !== "/",
							});

							return (
								<SidebarMenuItem key={item.title}>
									<SidebarMenuButton
										isActive={active}
										render={<Link to={item.to} />}
									>
										{item.title}
									</SidebarMenuButton>
									{item.items?.length ? (
										<SidebarMenuSub>
											{item.items.map((subItem) => {
												const subActive = !!matchRoute({
													to: subItem.to,
													fuzzy: false,
												});
												return (
													<SidebarMenuSubItem key={subItem.title}>
														<SidebarMenuSubButton
															isActive={subActive}
															render={<Link to={subItem.to} />}
														>
															{subItem.title}
														</SidebarMenuSubButton>
													</SidebarMenuSubItem>
												);
											})}
										</SidebarMenuSub>
									) : null}
								</SidebarMenuItem>
							);
						})}
					</SidebarMenu>
				</SidebarGroup>
			</SidebarContent>
			<SidebarRail />
		</Sidebar>
	);
}
