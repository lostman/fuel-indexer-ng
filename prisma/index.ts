import { PrismaClient } from '@prisma/client'

const prisma = new PrismaClient()

async function main() {
  const xs = await prisma.myComplexStruct.findMany( { include: { one: true, two: true } });
  console.log(xs)
}

main()
  .then(async () => {
    await prisma.$disconnect()
  })
  .catch(async (e) => {
    console.error(e)
    await prisma.$disconnect()
    process.exit(1)
  })
