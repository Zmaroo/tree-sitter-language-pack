import { Prisma } from "@prisma/client";
import multer from "multer";
import Stripe from "stripe";

import { AccountingService } from "../../services/AccountingService";
import { AccountingSyncConfigService } from "../../services/AccountingSyncConfigService";
import { AccountingSyncBatchService } from "../../services/AccountingSyncBatchService";
import { AccountingSyncBatchRunService } from "../../services/AccountingSyncBatchRunService";
import { AccountingSyncExecutionService } from "../../services/AccountingSyncExecutionService";
import { AccountingSyncHistoryService } from "../../services/AccountingSyncHistoryService";
import { AccountingSyncPreviewService } from "../../services/AccountingSyncPreviewService";
import { ApplicationService } from "../../services/ApplicationService";
import { AuthService } from "../../services/AuthService";
import { ChargeService } from "../../services/ChargeService";
import { DashboardService } from "../../services/DashboardService";
import { ExpenseService } from "../../services/ExpenseService";
import { FinancialPlanningService } from "../../services/FinancialPlanningService";
import { InvitationService } from "../../services/InvitationService";
import {
  LeaseDetailRecord,
  LeaseRecordService,
  LeaseRetrievabilityRecord,
} from "../../services/LeaseRecordService";
import { LeaseService } from "../../services/LeaseService";
import { LeaseSigningService } from "../../services/LeaseSigningService";
import { LateFeePolicyService } from "../../services/LateFeePolicyService";
import { PaymentMonitoringService } from "../../services/PaymentMonitoringService";
import { PaymentService } from "../../services/PaymentService";
import { QuickBooksService } from "../../services/QuickBooksService";
import { RecurringChargeService } from "../../services/RecurringChargeService";
import { ReminderService } from "../../services/ReminderService";
import { ReportService } from "../../services/ReportService";
import { StripeService } from "../../services/StripeService";
import { TenantCreditService } from "../../services/TenantCreditService";
import { TenantPortalService } from "../../services/TenantPortalService";
import { TenantService } from "../../services/TenantService";
import { TaxPackageService } from "../../services/TaxPackageService";

export type RouteServices = {
  accounting: AccountingService;
  accountingSyncBatchService: AccountingSyncBatchService;
  accountingSyncBatchRunService: AccountingSyncBatchRunService;
  accountingSyncConfigService: AccountingSyncConfigService;
  accountingSyncExecutionService: AccountingSyncExecutionService;
  accountingSyncHistoryService: AccountingSyncHistoryService;
  accountingSyncPreviewService: AccountingSyncPreviewService;
  applicationService: ApplicationService;
  authService: AuthService;
  chargeService: ChargeService;
  dashboardService: DashboardService;
  expenseService: ExpenseService;
  financialPlanningService: FinancialPlanningService;
  invitationService: InvitationService;
  lateFeePolicyService: LateFeePolicyService;
  leaseRecordService: LeaseRecordService;
  leaseService: LeaseService;
  leaseSigningService: LeaseSigningService;
  paymentMonitoringService: PaymentMonitoringService;
  paymentService: PaymentService;
  quickBooksService: QuickBooksService;
  recurringChargeService: RecurringChargeService;
  reminderService: ReminderService;
  reportService: ReportService;
  stripeService: StripeService | null;
  tenantCreditService: TenantCreditService;
  tenantPortalService: TenantPortalService;
  tenantService: TenantService;
  taxPackageService: TaxPackageService;
};

export type RouteHelpers = {
  assertOrgLease: (orgId: string, leaseId: string) => Promise<unknown>;
  assertOrgProperty: (orgId: string, propertyId: string) => Promise<unknown>;
  assertOrgRecurringChargeTemplate: (orgId: string, templateId: string) => Promise<unknown>;
  assertOrgRole: (orgId: string, roleId: string) => Promise<{ name: string }>;
  assertOrgTenant: (orgId: string, tenantId: string) => Promise<unknown>;
  assertOrgUnit: (orgId: string, unitId: string) => Promise<{ propertyId: string }>;
  assertOrgUser: (orgId: string, userId: string) => Promise<unknown>;
  assertOrgVendor: (orgId: string, vendorId: string) => Promise<unknown>;
  canAccessAnyTenant: (req: any) => boolean;
  leaseListInclude: Prisma.LeaseInclude;
  serializeApplication: (application: any) => unknown;
  serializeLeaseDetail: (result: LeaseDetailRecord) => unknown;
  serializeLeaseDocument: (document: any) => unknown;
  serializeLeaseListItem: (lease: any) => unknown;
  serializeLeaseSigningRequest: (request: any) => unknown;
  serializeTenantSummary: (result: any) => unknown;
};

export type RouteContext = {
  helpers: RouteHelpers;
  privilegedRoles: Set<string>;
  services: RouteServices;
  stripe: typeof Stripe;
  upload: multer.Multer;
};

export type { LeaseDetailRecord, LeaseRetrievabilityRecord };
