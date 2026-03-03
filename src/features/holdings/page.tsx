import { useQuery } from "@tanstack/react-query";
import { commands } from "../../bindings.ts";
import { columns } from "./columns.tsx";
import { DataTable } from "./data-table.tsx";

const HoldingsPage = () => {
	const { data } = useQuery({
		queryKey: ["holdings"],
		queryFn: async () => {
			const res = await commands.getHoldings();
			if (res.status === "error") throw new Error("oops");
			return res.data;
		},
	});
	console.log({ data });

	return (
		<div className="container mx-auto py-10">
			{data ? <DataTable columns={columns} data={data} /> : "Loading"}
		</div>
	);
};

export default HoldingsPage;
