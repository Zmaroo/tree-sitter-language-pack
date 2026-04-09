import {
  AccountingSyncAttemptType,
  AccountingSyncState,
  Prisma,
  PrismaClient,
} from "@prisma/client";

export class AccountingSyncHistoryService {
  constructor(private prisma: PrismaClient) {}

  async listQuickBooksAttempts(params: {
    orgId: string;
    limit?: number;
    offset?: number;
    resultState?: AccountingSyncState;
    attemptType?: AccountingSyncAttemptType;
  }) {
    const { orgId, limit = 30, offset = 0, resultState, attemptType } = params;
    const where: Prisma.AccountingJournalSyncAttemptWhereInput = {
      orgId,
      resultState,
      attemptType,
    };

    const items = await this.prisma.accountingJournalSyncAttempt.findMany({
      where,
      orderBy: {
        createdAt: "desc",
      },
      skip: offset,
      take: limit,
    });
    const totalCount = await this.prisma.accountingJournalSyncAttempt.count({ where });

    return {
      items,
      totalCount,
    };
  }

  async listQuickBooksNeedsAttention(params: {
    orgId: string;
    limit?: number;
    offset?: number;
  }) {
    const { orgId, limit = 25, offset = 0 } = params;
    const where: Prisma.AccountingJournalSyncWhereInput = {
      orgId,
      state: {
        in: [AccountingSyncState.FAILED, AccountingSyncState.STALE],
      },
    };

    const items = await this.prisma.accountingJournalSync.findMany({
      where,
      skip: offset,
      take: limit,
    });
    const totalCount = await this.prisma.accountingJournalSync.count({ where });
    const failedCount = await this.prisma.accountingJournalSync.count({
      where: {
        ...where,
        state: AccountingSyncState.FAILED,
      },
    });

    return {
      items,
      totalCount,
      failedCount,
    };
  }
}
