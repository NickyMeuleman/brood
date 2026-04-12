import {
	type ColumnDef,
	flexRender,
	getCoreRowModel,
	getSortedRowModel,
	type TableMeta,
	useReactTable,
} from "@tanstack/react-table";
import { ArrowLeftRightIcon, Settings2 } from "lucide-react";
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
import { cn, PERIOD_LABEL, PERIODS } from "@/lib/utils";
import { useUIStore } from "@/stores/ui";
import { Label } from "./Label";

interface DataTableProps<TData, TValue> {
	columns: ColumnDef<TData, TValue>[];
	data: TData[];
	meta?: TableMeta<TData>;
}

export function DataTable<TData, TValue>({
	columns,
	data,
	meta,
}: DataTableProps<TData, TValue>) {
	const {
		includeFees,
		setIncludeFees,
		displayInEur,
		setDisplayInEur,
		period,
		setPeriod,
		sorting,
		setSorting,
		columnVisibility,
		setColumnVisibility,
	} = useUIStore();

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
				<Tabs value={period} onValueChange={setPeriod}>
					<TabsList className="bg-muted p-1 text-muted-foreground">
						{PERIODS.map((period) => (
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
							checked={includeFees}
							onCheckedChange={setIncludeFees}
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
							checked={displayInEur}
							onCheckedChange={setDisplayInEur}
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
					<DropdownMenuContent align="end" className="w-auto">
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
									const { label, showPeriod } = column.columnDef.meta ?? {};
									const period = table.options.meta?.period;

									return (
										<DropdownMenuCheckboxItem
											key={column.id}
											className="capitalize"
											checked={column.getIsVisible()}
											onCheckedChange={(value) =>
												column.toggleVisibility(!!value)
											}
										>
											<Label
												label={label}
												period={period}
												showPeriod={showPeriod}
											/>
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
									const { label, showPeriod } = column.columnDef.meta ?? {};
									const period = table.options.meta?.period;

									return (
										<DropdownMenuCheckboxItem
											key={column.id}
											className="capitalize"
											checked={column.getIsVisible()}
											onCheckedChange={(value) =>
												column.toggleVisibility(!!value)
											}
										>
											<Label
												label={label}
												period={period}
												showPeriod={showPeriod}
											/>
										</DropdownMenuCheckboxItem>
									);
								})}
						</DropdownMenuGroup>
					</DropdownMenuContent>
				</DropdownMenu>
			</div>

			{displayInEur && (
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
											className={header.column.columnDef.meta?.cellClassName}
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
												cell.column.columnDef.meta?.cellClassName,
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
												footer.column.columnDef.meta?.cellClassName,
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
