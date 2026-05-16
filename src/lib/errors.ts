import type { AppError } from "../bindings";

export function getErrorMessage(error: AppError): string {
	switch (error.type) {
		case "NotFound":
			return "Data not found";
		case "Database":
			return `Database: ${error.data}`;
		case "Internal":
			return "An unexpected error occurred";
		case "Timeout":
			return `Timeout: ${error.data}`;
	}
}
