import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { routeTree } from "./routeTree.gen";
import "@/styles/global.css";
import type { RowData } from "@tanstack/react-table";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { Period, TotalsEUR } from "./bindings";

const router = createRouter({ routeTree });
const queryClient = new QueryClient();

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

declare module "@tanstack/react-table" {
	interface TableMeta<TData extends RowData> {
		totals?: TotalsEUR;
		period: Period;
	}
	interface ColumnMeta<TData extends RowData, TValue> {
		label?: string;
		hideByDefault?: boolean;
		cellClassName?: string;
		align?: "start" | "end";
    showPeriod?: boolean;
	}
}

const rootElement = document.getElementById("root") as HTMLElement;
if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(
		<StrictMode>
			<QueryClientProvider client={queryClient}>
				<TooltipProvider>
					<RouterProvider router={router} />
				</TooltipProvider>
			</QueryClientProvider>
		</StrictMode>,
	);
}
