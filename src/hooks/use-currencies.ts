import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useCurrencies() {
	return useQuery({
		queryKey: queryKeys.currencies,
		queryFn: async () => {
			const res = await commands.getCurrencies();
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
	});
}
