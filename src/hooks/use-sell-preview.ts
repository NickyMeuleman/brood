import { useQuery } from "@tanstack/react-query";
import { type CreateSellTradeInput, commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useSellPreview(input: CreateSellTradeInput) {
	return useQuery({
		queryKey: queryKeys.sellPreviewFor(
			input.listing_id,
			input.quantity,
			input.unit_price,
			input.executed_at,
			input.broker_fee ?? "",
			input.tob_fee ?? "",
		),
		queryFn: async () => {
			const res = await commands.previewSell(input);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		enabled:
			input.listing_id > 0 &&
			Number(input.quantity) > 0 &&
			Number(input.unit_price) > 0,
	});
}
