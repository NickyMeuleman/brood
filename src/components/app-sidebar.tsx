import { Link } from "@tanstack/react-router";
import { GalleryVerticalEndIcon } from "lucide-react";
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

const data = {
	navMain: [
		{
			title: "Holdings",
			to: "/",
		},
		{
			title: "Chart",
			to: "/chart",
		},
		{
			title: "Form",
			to: "/people",
		},
	],
};

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
	return (
		<Sidebar {...props}>
			<SidebarHeader>
				<SidebarMenu>
					<SidebarMenuItem>
						<SidebarMenuButton size="lg" render={<a href="#" />}>
							<div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
								<GalleryVerticalEndIcon className="size-4" />
							</div>
							<div className="flex flex-col gap-0.5 leading-none">
								<span className="font-medium">Brood</span>
								<span className="text-xs">Met beleg tussen</span>
							</div>
						</SidebarMenuButton>
					</SidebarMenuItem>
				</SidebarMenu>
			</SidebarHeader>
			<SidebarContent>
				<SidebarGroup>
					<SidebarMenu>
						{data.navMain.map((item) => (
							<SidebarMenuItem key={item.title}>
								<SidebarMenuButton
									className="data-[status=active]:bg-accent data-[status=active]:font-medium data-[status=active]:text-accent-foreground"
									render={<Link to={item.to} />}
								>
									{item.title}
								</SidebarMenuButton>
								{item.items?.length ? (
									<SidebarMenuSub>
										{item.items.map((item) => (
											<SidebarMenuSubItem key={item.title}>
												<SidebarMenuSubButton
													isActive={item.isActive}
													render={<a href={item.url} />}
												>
													{item.title}
												</SidebarMenuSubButton>
											</SidebarMenuSubItem>
										))}
									</SidebarMenuSub>
								) : null}
							</SidebarMenuItem>
						))}
					</SidebarMenu>
				</SidebarGroup>
			</SidebarContent>
			<SidebarRail />
		</Sidebar>
	);
}
