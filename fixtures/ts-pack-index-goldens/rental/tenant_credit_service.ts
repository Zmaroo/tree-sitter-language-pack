import { Prisma, PrismaClient } from "@prisma/client";

export class TenantCreditService {
  constructor(private prisma: PrismaClient) {}

  async createOverpaymentCredit(params: {
    amount: Prisma.Decimal;
  }) {
    return this.prisma.tenantCredit.create({
      data: params,
    });
  }

  async getUnappliedCreditBalance(orgId: string, tenantId: string) {
    const rows = await this.prisma.$queryRaw<Array<{ total: Prisma.Decimal }>>`
      SELECT COALESCE(SUM(unapplied_amount), 0) AS total
      FROM tenant_credits
      WHERE org_id = ${orgId}
        AND tenant_id = ${tenantId}
    `;

    return rows[0]?.total;
  }

  async applyAvailableCreditsForTenant(orgId: string, tenantId: string) {
    const credits = await this.prisma.tenantCredit.findMany({
      where: { orgId, tenantId },
    });

    const application = await this.prisma.tenantCreditApplication.create({
      data: { orgId, tenantId, amount: 1 },
    });

    await this.prisma.tenantCredit.update({
      where: { id: application.id },
      data: { memo: "applied" },
    });

    return credits;
  }
}
