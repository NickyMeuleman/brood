import { useMutation, useQueryClient } from "@tanstack/react-query";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { commands, type CreateBuyTradeInput } from "../bindings";

export function useBuy() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (input: CreateBuyTradeInput) => {
			const res = await commands.buy(input);
			if (res.status === "error") {
				throw new Error(getErrorMessage(res.error));
			}
			return res.data;
		},
		onMutate: () => {},
		onSuccess: () => {
			// queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
			// queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
		},
		onError: (e) => {},
	});
}
