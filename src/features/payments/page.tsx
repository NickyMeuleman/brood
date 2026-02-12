import { columns, type Payment } from "./columns";
import paymentData from "./data.ts";
import { DataTable } from "./data-table";

function getData(): Payment[] {
	return paymentData;
}

export default function DemoPage() {
	const data = getData();

	return (
		<div className="container mx-auto py-10">
			<DataTable columns={columns} data={data} />
		</div>
	);
}
