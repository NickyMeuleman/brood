import { useQuery } from "@tanstack/react-query";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { commands } from "../bindings";

export function usePrice(date: string, listing_id: number) {
	return useQuery({
		enabled: Boolean(listing_id && date),
		queryKey: queryKeys.priceFor(listing_id, date),
		queryFn: async () => {
			const res = await commands.getPrice(
				listing_id,
				date || new Date().toISOString(),
			);
			if (res.status === "error") {
				throw new Error(getErrorMessage(res.error));
			}
      
			return res.data;
		},
	});
}
