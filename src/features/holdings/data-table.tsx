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
import { Settings2 } from "lucide-react";
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
import {
	Table,
	TableBody,
	TableCell,
	TableFooter,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";

interface DataTableProps<TData, TValue> {
	columns: ColumnDef<TData, TValue>[];
	data: TData[];
	meta?: TableMeta<TData>;
	sorting: SortingState;
	onSortingChange: (
		updater: SortingState | ((prev: SortingState) => SortingState),
	) => void;
	columnVisibility: VisibilityState;
	onColumnVisibilityChange: (
		updater: VisibilityState | ((prev: VisibilityState) => VisibilityState),
	) => void;
}

export function DataTable<TData, TValue>({
	columns,
	data,
	meta,
	columnVisibility,
	onColumnVisibilityChange,
	sorting,
	onSortingChange,
}: DataTableProps<TData, TValue>) {
	const table = useReactTable({
		data,
		columns,
		getCoreRowModel: getCoreRowModel(),
		getSortedRowModel: getSortedRowModel(),
		onSortingChange,
		onColumnVisibilityChange,
		state: { sorting, columnVisibility },
		meta,
	});

	return (
		<div className="flex flex-col gap-4">
			<div>
				<div className="flex items-center justify-between">
					<div className="flex flex-1 items-center gap-2">filter input</div>
					<div className="flex items-center gap-2">
						<DropdownMenu>
							<DropdownMenuTrigger
								render={
									<Button
										variant="outline"
										size="sm"
										className="ml-auto hidden h-8 lg:flex"
									/>
								}
							>
								<span className="sr-only">Open menu</span>
								<Settings2 />
								View
							</DropdownMenuTrigger>
							<DropdownMenuContent align="end" className="w-37.5">
								<DropdownMenuGroup>
									<DropdownMenuLabel>Toggle columns</DropdownMenuLabel>
									<DropdownMenuSeparator />
									{table
										.getAllColumns()
										.filter(
											(column) =>
												typeof column.accessorFn !== "undefined" &&
												column.getCanHide(),
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
				</div>
			</div>
			<div className="overflow-hidden rounded-md border">
				<Table>
					<TableHeader>
						{table.getHeaderGroups().map((headerGroup) => (
							<TableRow key={headerGroup.id}>
								{headerGroup.headers.map((header) => {
									return (
										<TableHead key={header.id}>
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
										<TableCell key={cell.id}>
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
										<TableHead key={footer.id}>
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
