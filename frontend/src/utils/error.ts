export const getApiErrorMessage = (error: unknown, fallback: string): string => {
  if (typeof error !== "object" || error === null) {
    return fallback;
  }

  const maybeError = error as {
    response?: { data?: { message?: unknown; error?: unknown } | string };
    message?: unknown;
  };

  const responseData = maybeError.response?.data;
  const responseMessage =
    typeof responseData === "string"
      ? responseData
      : [responseData?.message, responseData?.error].find(
          (value): value is string =>
            typeof value === "string" && value.trim().length > 0,
        );
  if (responseMessage) {
    return responseMessage;
  }

  if (maybeError.response) {
    return fallback;
  }

  if (typeof maybeError.message === "string" && maybeError.message.trim().length > 0) {
    return maybeError.message;
  }

  return fallback;
};
