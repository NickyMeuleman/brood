import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useSell() {
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	return useMutation({
		mutationFn: async (input: any /*: CreateBuyTradeInput*/) => {
			// const res = await commands.buy(input);
			// if (res.status === "error") {
			// 	throw new Error(getErrorMessage(res.error));
			// }
			// return res.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
			queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
			toast.success("Trade recorded");
			navigate({ to: "/" });
		},
		onError: (e) => {
			toast.error("Failed to record trade", { description: e.message });
		},
	});
}
