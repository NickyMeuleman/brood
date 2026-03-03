import { type ColumnDef, createColumnHelper } from "@tanstack/react-table";
import { MoreHorizontal } from "lucide-react";
import type { Holding } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SortableHeaderButton } from "./SortableHeaderButton";

const columnHelper = createColumnHelper<Holding>();

const columnsArr = [
	columnHelper.accessor("ticker", {
		header: ({ column }) => (
			<SortableHeaderButton column={column} label="Ticker" />
		),
	}),
	columnHelper.accessor("name", {
		header: ({ column }) => (
			<SortableHeaderButton column={column} label="Name" />
		),
	}),
	columnHelper.accessor("quantity", {
		header: ({ column }) => (
			<SortableHeaderButton column={column} label="Quantity" align="end" />
		),
		cell: ({ row }) => {
			const quantity = parseFloat(row.getValue("quantity"));

			return <div className="text-right font-medium">{quantity}</div>;
		},
	}),
	columnHelper.accessor("current_price", {
		header: ({ column }) => (
			<SortableHeaderButton column={column} label="Unit Price" align="end" />
		),
		cell: ({ row }) => {
			const price = parseFloat(row.getValue("current_price"));
			const formatted = Intl.NumberFormat("nl-BE", {
				style: "currency",
				currency: row.original.currency_code,
			}).format(price);

			return <div className="text-right font-medium">{formatted}</div>;
		},
	}),
	columnHelper.accessor("market_value", {
		header: ({ column }) => (
			<SortableHeaderButton column={column} label="Total" align="end" />
		),
		cell: ({ row }) => {
			const price = parseFloat(row.getValue("market_value"));
			const formatted = Intl.NumberFormat("nl-BE", {
				style: "currency",
				currency: row.original.currency_code,
			}).format(price);

			return <div className="text-right font-medium">{formatted}</div>;
		},
		footer: ({ table }) => {
			const quantity = table
				.getFilteredRowModel()
				.rows.reduce(
					(acc, curr) => acc + Number(curr.getValue("market_value")),
					0,
				);
			const formatted = Intl.NumberFormat("nl-BE", {
				style: "currency",
				// todo: check if everything is EUR before showing
				currency: "EUR",
			}).format(quantity);

			return <div className="text-right font-bold">{formatted}</div>;
		},
	}),
	columnHelper.display({
		id: "actions",
		cell: ({ row }) => {
			const holding = row.original;

			return (
				<DropdownMenu>
					<DropdownMenuTrigger
						render={<Button variant="ghost" className="h-8 w-8 p-0" />}
					>
						<span className="sr-only">Open menu</span>
						<MoreHorizontal className="h-4 w-4" />
					</DropdownMenuTrigger>
					<DropdownMenuContent align="end">
						<DropdownMenuGroup>
							<DropdownMenuLabel>Actions</DropdownMenuLabel>
							<DropdownMenuItem
								onClick={() => {
									navigator.clipboard.writeText(holding.isin);
								}}
							>
								Copy ISIN
							</DropdownMenuItem>
						</DropdownMenuGroup>
					</DropdownMenuContent>
				</DropdownMenu>
			);
		},
	}),
];
// https://github.com/TanStack/table/issues/4382
export const columns = columnsArr as ColumnDef<Holding>[];
