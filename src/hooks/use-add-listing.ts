import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { type AddListingInput, commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useAddListing() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (input: AddListingInput) => {
			const res = await commands.addListingForm(input);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		onSuccess: (_, variables) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.listings });
			toast.success("Listing added", {
				description: `${variables.listing.ticker} on ${variables.listing.mic}`,
			});
		},
		onError: (e) => {
			toast.error("Failed to add listing", { description: e.message });
		},
	});
}
