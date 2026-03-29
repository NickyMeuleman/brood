import {
	type ColumnDef,
	flexRender,
	getCoreRowModel,
	getSortedRowModel,
	type SortingState,
	type TableMeta,
	useReactTable,
	type VisibilityState,
} from "@tanstack/react-table";
import { ArrowLeftRightIcon, Settings2 } from "lucide-react";
import { useState } from "react";
import type { Period } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import {
	Table,
	TableBody,
	TableCell,
	TableFooter,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn, PERIOD_LABEL } from "@/lib/utils";

export interface DisplayControls {
	includeFees: boolean;
	onIncludeFeesChange: (v: boolean) => void;
	displayInEur: boolean;
	onDisplayInEurChange: (v: boolean) => void;
	periods: Period[];
	period: Period;
	onPeriodChange: (v: Period) => void;
}

interface DataTableProps<TData, TValue> {
	columns: ColumnDef<TData, TValue>[];
	data: TData[];
	meta?: TableMeta<TData>;
	display: DisplayControls;
}

export function DataTable<TData, TValue>({
	columns,
	data,
	meta,
	display,
}: DataTableProps<TData, TValue>) {
	const [sorting, setSorting] = useState<SortingState>([
		{ id: "agg_identity", desc: false },
	]);
	const [columnVisibility, setColumnVisibility] = useState<VisibilityState>(
		() =>
			columns.reduce((acc, col) => {
				const id = col.id ?? (col as { accessorKey?: string }).accessorKey;
				if (id && col.meta?.hideByDefault) {
					acc[id] = false;
				}
				return acc;
			}, {} as VisibilityState),
	);

	const table = useReactTable({
		data,
		columns,
		getCoreRowModel: getCoreRowModel(),
		getSortedRowModel: getSortedRowModel(),
		onSortingChange: setSorting,
		onColumnVisibilityChange: setColumnVisibility,
		state: { sorting, columnVisibility },
		meta,
	});

	return (
		<div className="flex flex-col gap-3">
			<div className="flex items-center justify-between gap-4">
				<Tabs value={display.period} onValueChange={display.onPeriodChange}>
					<TabsList className="bg-muted p-1 text-muted-foreground">
						{display.periods.map((period) => (
							<TabsTrigger
								key={period}
								value={period}
								className="font-medium text-muted-foreground text-xs uppercase tracking-widest data-active:bg-primary data-active:text-primary-foreground data-active:hover:text-primary-foreground/80"
							>
								{PERIOD_LABEL[period]}
							</TabsTrigger>
						))}
					</TabsList>
				</Tabs>
				<FieldGroup className="flex flex-row justify-end">
					<Field orientation="horizontal" className="w-auto">
						<Switch
							id="toolbar-fees-toggle"
							checked={display.includeFees}
							onCheckedChange={display.onIncludeFeesChange}
						/>
						<FieldLabel
							htmlFor="toolbar-fees-toggle"
							className="cursor-pointer select-none font-normal text-sm"
						>
							Fees
						</FieldLabel>
					</Field>
					<Field orientation="horizontal" className="w-auto">
						<Switch
							id="toolbar-eur-toggle"
							checked={display.displayInEur}
							onCheckedChange={display.onDisplayInEurChange}
						/>
						<FieldLabel
							htmlFor="toolbar-eur-toggle"
							className="cursor-pointer select-none font-normal text-sm"
						>
							Convert to €
						</FieldLabel>
					</Field>
				</FieldGroup>
				<DropdownMenu>
					<DropdownMenuTrigger
						render={
							<Button variant="outline" size="sm" className="h-8 gap-1.5" />
						}
					>
						<span className="sr-only">Open menu</span>
						<Settings2 className="h-4 w-4" />
						<span className="hidden sm:inline">Columns</span>
					</DropdownMenuTrigger>
					<DropdownMenuContent align="end" className="w-40">
						<DropdownMenuGroup>
							<DropdownMenuLabel>Grouped</DropdownMenuLabel>
							{table
								.getAllColumns()
								.filter(
									(column) =>
										typeof column.accessorFn !== "undefined" &&
										column.getCanHide() &&
										column.id.startsWith("agg"),
								)
								.map((column) => {
									const label = column.columnDef.meta?.label ?? column.id;

									return (
										<DropdownMenuCheckboxItem
											key={column.id}
											className="capitalize"
											checked={column.getIsVisible()}
											onCheckedChange={(value) =>
												column.toggleVisibility(!!value)
											}
										>
											{label}
										</DropdownMenuCheckboxItem>
									);
								})}
						</DropdownMenuGroup>
						<DropdownMenuSeparator />
						<DropdownMenuGroup>
							<DropdownMenuLabel>Individual</DropdownMenuLabel>
							{table
								.getAllColumns()
								.filter(
									(column) =>
										typeof column.accessorFn !== "undefined" &&
										column.getCanHide() &&
										!column.id.startsWith("agg"),
								)
								.map((column) => {
									const label = column.columnDef.meta?.label ?? column.id;

									return (
										<DropdownMenuCheckboxItem
											key={column.id}
											className="capitalize"
											checked={column.getIsVisible()}
											onCheckedChange={(value) =>
												column.toggleVisibility(!!value)
											}
										>
											{label}
										</DropdownMenuCheckboxItem>
									);
								})}
						</DropdownMenuGroup>
					</DropdownMenuContent>
				</DropdownMenu>
			</div>

			{display.displayInEur && (
				<p className="flex items-center gap-1.5 text-muted-foreground text-xs">
					<ArrowLeftRightIcon className="h-3 w-3" />
					Values converted from local currency using the latest stored FX rate.
					Hover a converted value to see the original.
				</p>
			)}

			<div className="overflow-hidden rounded-md border">
				<Table>
					<TableHeader>
						{table.getHeaderGroups().map((headerGroup) => (
							<TableRow key={headerGroup.id}>
								{headerGroup.headers.map((header) => {
									return (
										<TableHead
											key={header.id}
											className={
												(header.column.columnDef.meta as any)?.cellClassName
											}
										>
											{header.isPlaceholder
												? null
												: flexRender(
														header.column.columnDef.header,
														header.getContext(),
													)}
										</TableHead>
									);
								})}
							</TableRow>
						))}
					</TableHeader>
					<TableBody>
						{table.getRowModel().rows?.length ? (
							table.getRowModel().rows.map((row) => (
								<TableRow
									key={row.id}
									data-state={row.getIsSelected() && "selected"}
								>
									{row.getVisibleCells().map((cell) => (
										<TableCell
											key={cell.id}
											className={cn(
												"tabular-nums tracking-tight",
												(
													cell.column.columnDef.meta as {
														cellClassName?: string;
													}
												)?.cellClassName,
											)}
										>
											{flexRender(
												cell.column.columnDef.cell,
												cell.getContext(),
											)}
										</TableCell>
									))}
								</TableRow>
							))
						) : (
							<TableRow>
								<TableCell
									colSpan={columns.length}
									className="h-24 text-center"
								>
									No results.
								</TableCell>
							</TableRow>
						)}
					</TableBody>
					<TableFooter>
						{table.getFooterGroups().map((footerGroup) => (
							<TableRow key={footerGroup.id}>
								{footerGroup.headers.map((footer) => {
									return (
										<TableHead
											key={footer.id}
											className={cn(
												"p-2 tabular-nums tracking-tight",
												(footer.column.columnDef.meta as any)?.cellClassName,
											)}
										>
											{footer.isPlaceholder
												? null
												: flexRender(
														footer.column.columnDef.footer,
														footer.getContext(),
													)}
										</TableHead>
									);
								})}
							</TableRow>
						))}
					</TableFooter>
				</Table>
			</div>
		</div>
	);
}
